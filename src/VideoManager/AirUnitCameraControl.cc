#include "AirUnitCameraControl.h"
#include "LinkInterface.h"
#include "MAVLinkProtocol.h"
#include "MultiVehicleManager.h"
#include "QGCLoggingCategory.h"

QGC_LOGGING_CATEGORY(AirUnitCameraControlLog, "qgc.videomanager.airunitcameracontrol")

namespace {
constexpr int kLegacyStreamInformationCameraIdOffset = 14;
constexpr int kLegacyStreamInformationMinimumLength = 16;

constexpr int kSwitchingNoticeMilliseconds = 15000;
}

AirUnitCameraControl::AirUnitCameraControl(QObject *parent)
    : QObject(parent)
{
    connect(MAVLinkProtocol::instance(), &MAVLinkProtocol::messageReceived, this, &AirUnitCameraControl::handleMessage);
    _noticeTimer.setSingleShot(true);
    connect(&_noticeTimer, &QTimer::timeout, this, [this]() { _showNotice(QString()); });
}

QString AirUnitCameraControl::inputName(int input)
{
    switch (input) {
    case 0: return tr("MIPI");
    case 1: return tr("HDMI");
    default: return tr("Input %1").arg(input + 1);
    }
}

bool AirUnitCameraControl::isCameraComponent(int componentId)
{
    return componentId >= MAV_COMP_ID_CAMERA && componentId <= MAV_COMP_ID_CAMERA6;
}

int AirUnitCameraControl::cameraIdFromLegacyStreamInformation(const mavlink_message_t &message)
{
    if (message.msgid != MAVLINK_MSG_ID_VIDEO_STREAM_INFORMATION || message.len < kLegacyStreamInformationMinimumLength) {
        return -1;
    }
    return _MAV_PAYLOAD(&message)[kLegacyStreamInformationCameraIdOffset];
}

void AirUnitCameraControl::handleMessage(LinkInterface *link, const mavlink_message_t &message)
{
    if (!isCameraComponent(message.compid)) {
        return;
    }
    if (message.msgid == MAVLINK_MSG_ID_HEARTBEAT) {
        if (MultiVehicleManager::instance()->getVehicleById(message.sysid)) {
            return;
        }
        const bool wasAvailable = available();
        _link = link;
        _systemId = message.sysid;
        _componentId = message.compid;
        if (!wasAvailable) {
            qCDebug(AirUnitCameraControlLog) << "Air unit camera found on system" << _systemId << "component" << _componentId;
            emit availableChanged();
            _requestStreamInformation();
        }
        return;
    }
    if (message.sysid != _systemId || message.compid != _componentId) {
        return;
    }
    if (message.msgid == MAVLINK_MSG_ID_COMMAND_ACK) {
        mavlink_command_ack_t ack;
        mavlink_msg_command_ack_decode(&message, &ack);
        if (ack.command != MAV_CMD_VIDEO_START_STREAMING) {
            return;
        }
        if (ack.result == MAV_RESULT_ACCEPTED || ack.result == MAV_RESULT_IN_PROGRESS) {
            _showNotice(QString());
        } else {
            qCWarning(AirUnitCameraControlLog) << "Air unit refused input" << _activeInput << "result" << ack.result;
            _showNotice(tr("Air unit refused %1").arg(inputName(_activeInput)));
            _setActiveInput(_previousInput);
        }
        return;
    }
    _setActiveInput(cameraIdFromLegacyStreamInformation(message));
}

void AirUnitCameraControl::_showNotice(const QString &text, int milliseconds)
{
    if (text == _notice) {
        return;
    }
    _notice = text;
    emit noticeChanged();
    if (!text.isEmpty()) {
        _noticeTimer.start(milliseconds);
    }
}

void AirUnitCameraControl::_setActiveInput(int input)
{
    if (input < 0 || input == _activeInput) {
        return;
    }
    _activeInput = input;
    emit activeInputChanged();
}

void AirUnitCameraControl::selectInput(int input)
{
    if (!available() || input < 0 || input >= kInputCount) {
        return;
    }
    qCDebug(AirUnitCameraControlLog) << "Selecting air unit input" << input;
    _previousInput = _activeInput;
    _setActiveInput(input);
    _showNotice(tr("Switching to %1…").arg(inputName(input)), kSwitchingNoticeMilliseconds);
    _sendCommand(MAV_CMD_VIDEO_START_STREAMING, static_cast<float>(input));
    _requestStreamInformation();
}

void AirUnitCameraControl::switchInput()
{
    selectInput((qMax(_activeInput, 0) + 1) % kInputCount);
}

void AirUnitCameraControl::_requestStreamInformation()
{
    _sendCommand(MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION, 0.0f);
}

void AirUnitCameraControl::_sendCommand(uint16_t command, float param1)
{
    if (!available()) {
        return;
    }
    mavlink_message_t message;
    mavlink_msg_command_long_pack_chan(MAVLinkProtocol::instance()->getSystemId(),
                                       MAVLinkProtocol::getComponentId(),
                                       _link->mavlinkChannel(),
                                       &message,
                                       _systemId,
                                       _componentId,
                                       command,
                                       0,
                                       param1, 0, 0, 0, 0, 0, 0);
    _sendMessage(_link, message);
}

void AirUnitCameraControl::_sendMessage(LinkInterface *link, const mavlink_message_t &message)
{
    if (!link || !link->isConnected()) {
        return;
    }
    uint8_t buffer[MAVLINK_MAX_PACKET_LEN];
    const int length = mavlink_msg_to_send_buffer(buffer, &message);
    link->writeBytesThreadSafe(reinterpret_cast<const char *>(buffer), length);
}
