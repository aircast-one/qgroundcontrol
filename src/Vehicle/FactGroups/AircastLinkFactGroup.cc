#include "AircastLinkFactGroup.h"
#include "Vehicle.h"

AircastLinkFactGroup::AircastLinkFactGroup(QObject *parent)
    : FactGroup(1000, QStringLiteral(":/json/Vehicle/AircastLinkFact.json"), parent)
{
    _addFact(&_qualityFact);
    _addFact(&_radioTypeFact);
    _addFact(&_statusFact);
    _addFact(&_videoBitrateFact);
}

void AircastLinkFactGroup::handleMessage(Vehicle *vehicle, const mavlink_message_t &message)
{
    Q_UNUSED(vehicle);

    if (message.msgid == MAVLINK_MSG_ID_CELLULAR_STATUS) {
        _handleCellularStatus(message);
    }
}

void AircastLinkFactGroup::_handleCellularStatus(const mavlink_message_t &message)
{
    mavlink_cellular_status_t cellular{};
    mavlink_msg_cellular_status_decode(&message, &cellular);

    const uint32_t bitrateKbps = (cellular.link_rx_rate == UINT32_MAX) ? 0 : qRound(cellular.link_rx_rate * 1024.0 * 8.0 / 1000.0);

    quality()->setRawValue(cellular.quality);
    radioType()->setRawValue(cellular.type);
    status()->setRawValue(cellular.status);
    videoBitrate()->setRawValue(bitrateKbps);

    _remember(_qualityHistory, (cellular.quality == kQualityUnknown) ? QVariant(-1) : QVariant(static_cast<int>(cellular.quality)));
    _remember(_bitrateHistory, bitrateKbps);
    emit historyChanged();

    _setTelemetryAvailable(true);
}

void AircastLinkFactGroup::_remember(QVariantList &history, const QVariant &value)
{
    history.append(value);
    if (history.size() > kHistoryLength) {
        history.removeFirst();
    }
}
