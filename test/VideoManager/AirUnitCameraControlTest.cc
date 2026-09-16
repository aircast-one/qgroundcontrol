#include "AirUnitCameraControlTest.h"
#include "AirUnitCameraControl.h"
#include "LinkInterface.h"
#include "UDPLink.h"

#include <QtTest/QTest>

namespace {

class FakeLink : public LinkInterface
{
public:
    FakeLink()
        : LinkInterface(_makeConfig())
    {
        _allocateMavlinkChannel();
    }

    ~FakeLink() override { _freeMavlinkChannel(); }

    void disconnect() final {}
    bool isConnected() const final { return true; }

private:
    static SharedLinkConfigurationPtr &_makeConfig()
    {
        static SharedLinkConfigurationPtr config = std::make_shared<UDPConfiguration>(QStringLiteral("fake"));
        return config;
    }

    void _writeBytes(const QByteArray &) final {}
    bool _connect() final { return true; }
};

class RecordingControl : public AirUnitCameraControl
{
public:
    QList<mavlink_command_long_t> commands;

protected:
    void _sendMessage(LinkInterface *, const mavlink_message_t &message) final
    {
        if (message.msgid == MAVLINK_MSG_ID_COMMAND_LONG) {
            mavlink_command_long_t command;
            mavlink_msg_command_long_decode(&message, &command);
            commands.append(command);
        }
    }
};

mavlink_message_t cameraHeartbeat(uint8_t sysid = 42, uint8_t compid = MAV_COMP_ID_CAMERA)
{
    mavlink_message_t message;
    mavlink_msg_heartbeat_pack_chan(sysid, compid, 0, &message, MAV_TYPE_GENERIC, MAV_AUTOPILOT_INVALID, 0, 0, MAV_STATE_ACTIVE);
    return message;
}

mavlink_message_t legacyStreamInformation(uint8_t cameraId, uint8_t sysid = 42, uint8_t compid = MAV_COMP_ID_CAMERA)
{
    mavlink_message_t message;
    memset(&message, 0, sizeof(message));
    message.sysid = sysid;
    message.compid = compid;
    message.msgid = MAVLINK_MSG_ID_VIDEO_STREAM_INFORMATION;
    message.len = 246;
    uint8_t *payload = reinterpret_cast<uint8_t *>(_MAV_PAYLOAD_NON_CONST(&message));
    const float framerate = 30.0f;
    memcpy(payload, &framerate, sizeof(framerate));
    payload[14] = cameraId;
    const char uri[] = "rtsp://192.168.0.10:8554/H264Video";
    memcpy(payload + 16, uri, sizeof(uri));
    return message;
}

mavlink_message_t startStreamingAck(uint8_t result, uint8_t sysid = 42, uint8_t compid = MAV_COMP_ID_CAMERA)
{
    mavlink_message_t message;
    mavlink_msg_command_ack_pack_chan(sysid, compid, 0, &message, MAV_CMD_VIDEO_START_STREAMING, result, 0, 0, 255, 190);
    return message;
}

} // namespace

void AirUnitCameraControlTest::_cameraHeartbeatMakesItAvailable()
{
    FakeLink link;
    RecordingControl control;
    QVERIFY(!control.available());

    control.handleMessage(&link, cameraHeartbeat(42, MAV_COMP_ID_AUTOPILOT1));
    QVERIFY(!control.available());

    control.handleMessage(&link, cameraHeartbeat());
    QVERIFY(control.available());
    QCOMPARE(control.commands.size(), 1);
    QCOMPARE(control.commands.last().command, static_cast<uint16_t>(MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION));
    QCOMPARE(control.commands.last().target_system, static_cast<uint8_t>(42));
    QCOMPARE(control.commands.last().target_component, static_cast<uint8_t>(MAV_COMP_ID_CAMERA));
}

void AirUnitCameraControlTest::_legacyStreamInformationReportsActiveInput()
{
    FakeLink link;
    RecordingControl control;
    control.handleMessage(&link, cameraHeartbeat());
    QCOMPARE(control.activeInput(), -1);

    control.handleMessage(&link, legacyStreamInformation(1));
    QCOMPARE(control.activeInput(), 1);

    control.handleMessage(&link, legacyStreamInformation(0, 7));
    QCOMPARE(control.activeInput(), 1);

    control.handleMessage(&link, legacyStreamInformation(0));
    QCOMPARE(control.activeInput(), 0);
}

void AirUnitCameraControlTest::_selectInputSendsStartStreamingToTheCamera()
{
    FakeLink link;
    RecordingControl control;
    control.selectInput(1);
    QVERIFY(control.commands.isEmpty());

    control.handleMessage(&link, cameraHeartbeat());
    control.commands.clear();
    control.selectInput(1);
    QCOMPARE(control.commands.size(), 2);
    QCOMPARE(control.commands.first().command, static_cast<uint16_t>(MAV_CMD_VIDEO_START_STREAMING));
    QCOMPARE(control.commands.first().param1, 1.0f);
    QCOMPARE(control.commands.first().target_system, static_cast<uint8_t>(42));
    QCOMPARE(control.commands.first().target_component, static_cast<uint8_t>(MAV_COMP_ID_CAMERA));
    QCOMPARE(control.commands.last().command, static_cast<uint16_t>(MAV_CMD_REQUEST_VIDEO_STREAM_INFORMATION));

    control.commands.clear();
    control.selectInput(AirUnitCameraControl::kInputCount);
    QVERIFY(control.commands.isEmpty());
}

void AirUnitCameraControlTest::_switchInputCyclesThroughInputs()
{
    FakeLink link;
    RecordingControl control;
    control.handleMessage(&link, cameraHeartbeat());
    control.handleMessage(&link, legacyStreamInformation(0));
    control.commands.clear();

    control.switchInput();
    QCOMPARE(control.commands.first().param1, 1.0f);

    control.handleMessage(&link, legacyStreamInformation(1));
    control.commands.clear();
    control.switchInput();
    QCOMPARE(control.commands.first().param1, 0.0f);
}

void AirUnitCameraControlTest::_switchInputAlternatesWithoutStreamInformation()
{
    FakeLink link;
    RecordingControl control;
    control.handleMessage(&link, cameraHeartbeat());
    control.commands.clear();

    control.switchInput();
    QCOMPARE(control.commands.first().param1, 1.0f);
    QCOMPARE(control.activeInput(), 1);

    control.commands.clear();
    control.switchInput();
    QCOMPARE(control.commands.first().param1, 0.0f);
    QCOMPARE(control.activeInput(), 0);
}

void AirUnitCameraControlTest::_refusedSwitchRevertsTheInput()
{
    FakeLink link;
    RecordingControl control;
    control.handleMessage(&link, cameraHeartbeat());
    control.handleMessage(&link, legacyStreamInformation(1));

    control.selectInput(0);
    QCOMPARE(control.activeInput(), 0);

    control.handleMessage(&link, startStreamingAck(MAV_RESULT_FAILED));
    QCOMPARE(control.activeInput(), 1);

    control.selectInput(0);
    control.handleMessage(&link, startStreamingAck(MAV_RESULT_ACCEPTED));
    QCOMPARE(control.activeInput(), 0);
}

void AirUnitCameraControlTest::_inputsAreNamedAndRefusalIsAnnounced()
{
    QCOMPARE(AirUnitCameraControl::inputName(0), QStringLiteral("MIPI"));
    QCOMPARE(AirUnitCameraControl::inputName(1), QStringLiteral("HDMI"));

    FakeLink link;
    RecordingControl control;
    control.handleMessage(&link, cameraHeartbeat());
    control.handleMessage(&link, legacyStreamInformation(0));
    QCOMPARE(control.activeInputName(), QStringLiteral("MIPI"));
    QVERIFY(control.notice().isEmpty());

    control.selectInput(1);
    control.handleMessage(&link, startStreamingAck(MAV_RESULT_FAILED));
    QCOMPARE(control.notice(), QStringLiteral("Air unit refused HDMI"));
    QCOMPARE(control.activeInputName(), QStringLiteral("MIPI"));
}
