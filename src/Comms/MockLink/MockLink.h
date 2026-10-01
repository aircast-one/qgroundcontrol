#pragma once

#include <QtCore/QElapsedTimer>
#include <QtCore/QMap>
#include <QtCore/QMutex>
#include <QtCore/QSet>
#include <QtPositioning/QGeoCoordinate>
#include <array>
#include <atomic>

#include "LinkInterface.h"
#include "MAVLinkEnums.h"
#include "MAVLinkMessageType.h"
#include "MockConfiguration.h"
#include "MockLinkMissionItemHandler.h"
#include "MockLinkPX4Calibration.h"
#include "PX4/px4_custom_mode.h"
#include "QGCMAVLinkTypes.h"

class MockLinkCamera;
class MockLinkFTP;
class MockLinkGimbal;
class MockLinkWorker;
class MockVideoStreamServer;
class QThread;

class MockLink : public LinkInterface
{
    Q_OBJECT
    friend class MockLinkFTP;

public:
    explicit MockLink(SharedLinkConfigurationPtr& config, QObject* parent = nullptr);
    virtual ~MockLink();

    void run1HzTasks();
    void run10HzTasks();
    void run500HzTasks();
    void sendStatusTextMessages();

    bool shouldSendStatusText() const { return _sendStatusText; }

    bool isConnected() const final { return _connected; }

    void disconnect() final;

    Q_INVOKABLE void setCommLost(bool commLost) { _commLost = commLost; }

    void setBooting(bool booting) { _booting = booting; }

    Q_INVOKABLE void simulateConnectionRemoved();

    int vehicleId() const { return _vehicleSystemId; }

    MAV_AUTOPILOT getFirmwareType() const { return _firmwareType; }

    double vehicleLatitude() const { return _vehicleLatitude; }

    double vehicleLongitude() const { return _vehicleLongitude; }

    double vehicleAltitudeAMSL() const { return _vehicleAltitudeAMSL; }

    bool signingEnabled() const { return _signingEnabled; }

    void respondWithMavlinkMessage(const mavlink_message_t& msg);

    void sendStatusTextMessage(uint8_t severity, const QString& text);

    void setCalibrationPose(MockLinkPX4Calibration::Pose pose) const { _mockLinkPX4Calibration->setPose(pose); }

    MockLinkFTP* mockLinkFTP() const;

    void setArmed(bool armed)
    {
        if (armed)
            _mavBaseMode |= MAV_MODE_FLAG_SAFETY_ARMED;
        else
            _mavBaseMode &= ~MAV_MODE_FLAG_SAFETY_ARMED;
    }

    bool armed() const { return (_mavBaseMode & MAV_MODE_FLAG_SAFETY_ARMED) != 0; }

    void setMissionItemFailureMode(MockLinkMissionItemHandler::FailureMode_t failureMode,
                                   MAV_MISSION_RESULT failureAckResult) const
    {
        _missionItemHandler->setFailureMode(failureMode, failureAckResult);
    }

    void sendUnexpectedMissionAck(MAV_MISSION_RESULT ackType) const
    {
        _missionItemHandler->sendUnexpectedMissionAck(ackType);
    }

    void sendUnexpectedMissionItem() const { _missionItemHandler->sendUnexpectedMissionItem(); }

    void sendUnexpectedMissionRequest() const { _missionItemHandler->sendUnexpectedMissionRequest(); }

    void sendUnexpectedCommandAck(MAV_CMD command, MAV_RESULT ackResult);

    void resetMissionItemHandler() const { _missionItemHandler->reset(); }

    void loadSimpleMultirotorMission() const { _missionItemHandler->loadSimpleMultirotorMission(); }

    QString logDownloadFile() const { return _logDownloadFilename; }

    uint8_t outgoingMavlinkChannel() const { return _outgoingMavlinkChannel; }

    void clearReceivedMavCommandCounts()
    {
        _receivedMavCommandCountMap.clear();
        _receivedMavCommandByCompCountMap.clear();
        _receivedRequestMessageByCompAndMsgCountMap.clear();
    }

    int receivedMavCommandCount(MAV_CMD command) const { return _receivedMavCommandCountMap.value(command, 0); }

    int receivedMavCommandCount(MAV_CMD command, int compId) const
    {
        return _receivedMavCommandByCompCountMap.value(command).value(compId, 0);
    }

    int receivedRequestMessageCount(int compId, int messageId) const
    {
        return _receivedRequestMessageByCompAndMsgCountMap.value(compId).value(messageId, 0);
    }

    void clearReceivedRequestMessageCounts()
    {
        _receivedRequestMessageCountMap.clear();
        _receivedRequestMessageByCompAndMsgCountMap.clear();
    }

    int receivedRequestMessageCount(uint32_t messageId) const
    {
        return _receivedRequestMessageCountMap.value(messageId, 0);
    }

    void clearReceivedMavlinkMessageCounts()
    {
        _receivedMavlinkMessageCountMap.clear();
        _lastReceivedMavlinkMessageMap.clear();
        _hashCheckRequestCount = 0;
        _missionItemHandler->clearRequestListCounts();
    }

    int receivedMavlinkMessageCount(uint32_t messageId) const
    {
        return _receivedMavlinkMessageCountMap.value(messageId, 0);
    }

    bool lastReceivedMavlinkMessage(uint32_t messageId, mavlink_message_t& message) const
    {
        if (!_lastReceivedMavlinkMessageMap.contains(messageId)) {
            return false;
        }
        message = _lastReceivedMavlinkMessageMap.value(messageId);
        return true;
    }

    int receivedMissionRequestListCount(MAV_MISSION_TYPE type) const
    {
        return _missionItemHandler->requestListCount(type);
    }

    enum RequestMessageFailureMode_t
    {
        FailRequestMessageNone,
        FailRequestMessageCommandAcceptedMsgNotSent,
        FailRequestMessageCommandUnsupported,
        FailRequestMessageCommandNoResponse,
    };

    void setRequestMessageFailureMode(RequestMessageFailureMode_t failureMode)
    {
        _requestMessageFailureMode = failureMode;
    }

    void setRequestMessageNoResponse(uint32_t messageId, bool noResponse = true)
    {
        QMutexLocker locker(&_requestMessageNoResponseMutex);
        if (noResponse) {
            _requestMessageNoResponseIds.insert(messageId);
        } else {
            _requestMessageNoResponseIds.remove(messageId);
        }
    }

    enum ParamSetFailureMode_t
    {
        FailParamSetNone,
        FailParamSetNoAck,
        FailParamSetFirstAttemptNoAck,
        FailParamSetParamError,
    };

    void setParamSetFailureMode(ParamSetFailureMode_t mode)
    {
        _paramSetFailureMode = mode;
        _paramSetFailureFirstAttemptPending = (mode == FailParamSetFirstAttemptNoAck);
    }

    enum ParamRequestReadFailureMode_t
    {
        FailParamRequestReadNone,
        FailParamRequestReadNoResponse,
        FailParamRequestReadFirstAttemptNoResponse,
        FailParamRequestReadParamError,
    };

    void setParamRequestReadFailureMode(ParamRequestReadFailureMode_t mode)
    {
        _paramRequestReadFailureMode = mode;
        _paramRequestReadFailureFirstAttemptPending = (mode == FailParamRequestReadFirstAttemptNoResponse);
    }

    void setHashCheckNoResponse(bool noResponse) { _hashCheckNoResponse = noResponse; }

    void setResetSysAutostartOnParamReset(bool reset) { _resetSysAutostartOnParamReset = reset; }

    int hashCheckRequestCount() const { return _hashCheckRequestCount; }

    void setMockParamValue(int componentId, const QString& paramName, float value);

    void startAPMStaleFailedMagCalReportStreaming();

    bool apmStaleFailedMagCalReportStreamingActive() const;

    void setAPMMagCalStartFailureMode(bool fail) { _apmMagCalStartFailureMode = fail; }

    void setInt32ParamValue(int componentId, const QString& paramName, int32_t value)
    {
        _mapParamName2Value[componentId][paramName] = QVariant::fromValue(value);
    }

    QVariant paramValue(int componentId, const QString& paramName) const
    {
        return _mapParamName2Value.value(componentId).value(paramName);
    }

    void setRemoteIDArmStatus(uint8_t status, const QString& error);

    static MockLink* startPX4MockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);
    static MockLink* startPX4MockLinkWithMission(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone);
    static MockLink* startGenericMockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);
    static MockLink* startNoInitialConnectMockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone);
    static MockLink* startAPMArduCopterMockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);
    static MockLink* startAPMArduPlaneMockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);
    static MockLink* startAPMArduSubMockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);
    static MockLink* startAPMArduRoverMockLink(
        MockConfiguration::Options options = MockConfiguration::OptionNone,
        MockConfiguration::FailureMode_t failureMode = MockConfiguration::FailNone,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);

    void servedVideoStream(MockConfiguration::VideoStreamType& type, QString& uri) const;

    MockConfiguration::VideoStreamType requestedVideoStreamType() const { return _requestedVideoStreamType; }

    static constexpr MAV_CMD MAV_CMD_MOCKLINK_ALWAYS_RESULT_ACCEPTED = MAV_CMD_USER_1;
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_ALWAYS_RESULT_FAILED = MAV_CMD_USER_2;
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_SECOND_ATTEMPT_RESULT_ACCEPTED = MAV_CMD_USER_3;
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_SECOND_ATTEMPT_RESULT_FAILED = MAV_CMD_USER_4;
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_NO_RESPONSE = MAV_CMD_USER_5;
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_NO_RESPONSE_NO_RETRY = static_cast<MAV_CMD>(MAV_CMD_USER_5 + 1);
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_RESULT_IN_PROGRESS_ACCEPTED = static_cast<MAV_CMD>(MAV_CMD_USER_5 + 2);
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_RESULT_IN_PROGRESS_FAILED = static_cast<MAV_CMD>(MAV_CMD_USER_5 + 3);
    static constexpr MAV_CMD MAV_CMD_MOCKLINK_RESULT_IN_PROGRESS_NO_ACK = static_cast<MAV_CMD>(MAV_CMD_USER_5 + 4);

signals:
    void writeBytesQueuedSignal(const QByteArray& bytes);
    void highLatencyTransmissionEnabledChanged(bool highLatencyTransmissionEnabled);

private slots:
    void _writeBytes(const QByteArray& bytes) final;
    void _writeBytesQueued(const QByteArray& bytes);

private:
    typedef struct
    {
        const char* name;
        uint8_t standard_mode;
        uint32_t custom_mode;
        bool canBeSet;
        bool advanced;
    } FlightMode_t;

    bool _connect() final;
    bool _allocateMavlinkChannel() final;
    void _freeMavlinkChannel() final;

    bool _incomingMavlinkChannelIsSet() const;
    bool _outgoingMavlinkChannelIsSet() const;

    void _loadParams();
    void _resetParamsToDefaults();
    void _applyAPMFreshFlashState();

    float _floatUnionForParam(int componentId, const QString& paramName);
    uint32_t _computeParamHash(int componentId) const;
    void _setParamFloatUnionIntoMap(int componentId, const QString& paramName, float paramFloat);

    void _handleIncomingNSHBytes(const char* bytes, int cBytes);
    void _handleIncomingMavlinkBytes(const uint8_t* bytes, int cBytes);
    void _updateIncomingMessageCounts(const mavlink_message_t& msg);
    void _handleIncomingMavlinkMsg(const mavlink_message_t& msg);
    void _handleHeartBeat(const mavlink_message_t& msg);
    void _handleSetMode(const mavlink_message_t& msg);
    void _handleParamRequestList(const mavlink_message_t& msg);
    void _handleParamSet(const mavlink_message_t& msg);
    void _handleParamRequestRead(const mavlink_message_t& msg);
    void _handleFTP(const mavlink_message_t& msg);
    void _handleCommandLong(const mavlink_message_t& msg);
    void _handleCommandInt(const mavlink_message_t& msg);
    void _handleInProgressCommandLong(const mavlink_command_long_t& request);
    void _handleCommandLongSetMessageInterval(const mavlink_command_long_t& request, bool& acccepted);
    void _handleManualControl(const mavlink_message_t& msg);
    void _handleRCChannelsOverride(const mavlink_message_t& msg);
    void _handlePreFlightCalibration(const mavlink_command_long_t& request);
    void _handleTakeoff(const mavlink_command_long_t& request);
    void _handleLogRequestList(const mavlink_message_t& msg);
    void _handleLogErase(const mavlink_message_t& msg);
    void _handleLogRequestData(const mavlink_message_t& msg);
    void _handleParamMapRC(const mavlink_message_t& msg);
    void _handleSetupSigning(const mavlink_message_t& msg);
    void _sendParamError(int componentId, const char* paramId, int16_t paramIndex, uint8_t errorCode);
    void _handleRequestMessage(const mavlink_command_long_t& request, bool& accepted, bool& noAck);
    void _handleRequestMessageAutopilotVersion(const mavlink_command_long_t& request, bool& accepted);
    void _handleRequestMessageDebug(const mavlink_command_long_t& request, bool& accepted, bool& noAck);
    void _handleRequestMessageAvailableModes(const mavlink_command_long_t& request, bool& accepted);

    void _sendHeartBeat();
    void _sendHighLatency2();
    void _sendHomePosition();
    void _sendGpsRawInt();
    void _sendGlobalPositionInt();
    void _sendExtendedSysState();
    void _sendVibration();
    void _sendSysStatus();
    void _sendBatteryStatus();
    void _sendNamedValueFloats();
    void _sendDistanceSensors();
    void _sendChunkedStatusText(uint16_t chunkId, bool missingChunks);
    void _sendStatusTextMessages();
    void _respondWithAutopilotVersion();
    void _sendRCChannels();
    void _sendADSBVehicles();
    void _sendGeneralMetaData();
    void _sendRemoteIDArmStatus();
    void _sendEscInfo();
    void _sendEscStatus();
    void _sendRadioStatus();
    void _sendAvailableModesMonitor();
    void _sendAttitudeQuaternion();
    void _sendAttitudeTarget();
    void _sendLocalPositionNed();
    void _sendPositionTargetLocalNed();

    void _paramRequestListWorker();
    void _logDownloadWorker();
    void _availableModesWorker();
    void _apmCompassCalWorker();
    void _apmAccelCalWorker();
    void _sendAvailableMode(uint8_t modeIndexOneBased);
    int _availableModesCount() const;
    void _moveADSBVehicle(int vehicleIndex);

    static MockLink* _startMockLinkWorker(
        const QString& configName, MAV_AUTOPILOT firmwareType, MAV_TYPE vehicleType, MockConfiguration::Options options,
        MockConfiguration::FailureMode_t failureMode,
        MockConfiguration::VideoStreamType videoStreamType = MockConfiguration::VideoStreamNone);
    static MockLink* _startMockLink(MockConfiguration* mockConfig);

    void _startVideoStreamServer();
    void _stopVideoStreamServer();

    static QString _createRandomFile(uint32_t byteCount);
    QString _createLogContentsFile(const QString& logName);

    QThread* _workerThread = nullptr;
    MockLinkWorker* _worker = nullptr;

    const MockConfiguration* _mockConfig = nullptr;
    const MAV_AUTOPILOT _firmwareType = MAV_AUTOPILOT_PX4;
    const MAV_TYPE _vehicleType = MAV_TYPE_QUADROTOR;
    const bool _sendStatusText = false;
    const bool _apmStartFreshParams = false;
    const bool _enableCamera = false;
    const bool _enableGimbal = false;
    const bool _enableProximity = false;
    const MockConfiguration::FailureMode_t _failureMode = MockConfiguration::FailNone;
    const bool _stayMavlinkV1 = false;
    const bool _ftpCapability = false;
    const uint8_t _vehicleSystemId = 0;
    const double _vehicleLatitude = 0.0;
    const double _vehicleLongitude = 0.0;
    const uint16_t _boardVendorId = 0;
    const uint16_t _boardProductId = 0;
    MockLinkMissionItemHandler* const _missionItemHandler = nullptr;
    MockLinkCamera* const _mockLinkCamera = nullptr;
    MockLinkGimbal* const _mockLinkGimbal = nullptr;
    MockLinkPX4Calibration* const _mockLinkPX4Calibration = nullptr;
    MockLinkFTP* const _mockLinkFTP = nullptr;

    const MockConfiguration::VideoStreamType _requestedVideoStreamType = MockConfiguration::VideoStreamNone;
    MockVideoStreamServer* _videoStreamServer = nullptr;
    mutable QMutex _videoStreamMutex;
    MockConfiguration::VideoStreamType _servedVideoStreamType = MockConfiguration::VideoStreamNone;
    QString _videoStreamUri;

    uint8_t _incomingMavlinkChannel = std::numeric_limits<uint8_t>::max();
    QMutex _incomingMavlinkMutex;
    uint8_t _outgoingMavlinkChannel = std::numeric_limits<uint8_t>::max();

    mavlink_signing_t _mockSigning{};
    mavlink_signing_streams_t _mockSigningStreams{};

    bool _connected = false;
    bool _inNSH = false;

    uint8_t _mavBaseMode = MAV_MODE_FLAG_MANUAL_INPUT_ENABLED | MAV_MODE_FLAG_CUSTOM_MODE_ENABLED;
    uint32_t _mavCustomMode = PX4CustomMode::MANUAL;

    QElapsedTimer _runningTime;
    static constexpr int kTestParamRequestListBatch = 25;
    static constexpr int32_t _batteryMaxTimeRemaining = 15 * 60;
    int8_t _battery1PctRemaining = 100;
    int32_t _battery1TimeRemaining = _batteryMaxTimeRemaining;
    MAV_BATTERY_CHARGE_STATE _battery1ChargeState = MAV_BATTERY_CHARGE_STATE_OK;
    int8_t _battery2PctRemaining = 100;
    int32_t _battery2TimeRemaining = _batteryMaxTimeRemaining;
    MAV_BATTERY_CHARGE_STATE _battery2ChargeState = MAV_BATTERY_CHARGE_STATE_OK;

    double _vehicleAltitudeAMSL = _defaultVehicleHomeAltitude;
    std::atomic<bool> _commLost = false;
    std::atomic<bool> _booting = false;
    bool _mavlinkV2Upgraded = false;
    bool _signingEnabled = false;
    bool _highLatencyTransmissionEnabled = true;

    int _sendHomePositionDelayCount = 10;
    int _sendGPSPositionDelayCount = 100;

    int _currentParamRequestListComponentIndex = -1;
    int _currentParamRequestListParamIndex = -1;
    QList<int> _paramRequestListComponentIds;
    QStringList _paramRequestListParamNames;
    QMutex _paramRequestListMutex;

    int _availableModesWorkerNextModeIndex = 0;
    QMutex _availableModesWorkerMutex;
    uint8_t _availableModesMonitorSeqNumber = 0;

    QString _logDownloadFilename;
    bool _logsErased = false;
    uint16_t _logDownloadId = 0;
    uint32_t _logDownloadSize = 0;
    uint32_t _logDownloadCurrentOffset = 0;
    uint32_t _logDownloadBytesRemaining = 0;
    QMutex _logDownloadMutex;

    RequestMessageFailureMode_t _requestMessageFailureMode = FailRequestMessageNone;
    mutable QMutex _requestMessageNoResponseMutex;
    QSet<uint32_t> _requestMessageNoResponseIds;
    ParamSetFailureMode_t _paramSetFailureMode = FailParamSetNone;
    bool _paramSetFailureFirstAttemptPending = false;
    ParamRequestReadFailureMode_t _paramRequestReadFailureMode = FailParamRequestReadNone;
    bool _paramRequestReadFailureFirstAttemptPending = false;
    bool _hashCheckNoResponse = false;
    int _hashCheckRequestCount = 0;
    bool _paramRequestListHashCheckSent = false;
    bool _resetSysAutostartOnParamReset = false;

    QMutex _remoteIDArmStatusMutex;
    uint8_t _remoteIDArmStatus = MAV_ODID_ARM_STATUS_GOOD_TO_ARM;
    QString _remoteIDArmStatusError = QStringLiteral("No Error");

    mutable QMutex _apmCompassCalMutex;
    int _apmCompassCalProgress = -1;
    int _apmCompassCalTickCount = 0;
    bool _apmStaleFailedMagCalReportStreaming = false;
    bool _apmMagCalStartFailureMode = false;

    QMutex _apmAccelCalMutex;
    static constexpr ACCELCAL_VEHICLE_POS kAPMAccelCalPosSequence[] = {
        ACCELCAL_VEHICLE_POS_LEVEL,    ACCELCAL_VEHICLE_POS_LEFT,   ACCELCAL_VEHICLE_POS_RIGHT,
        ACCELCAL_VEHICLE_POS_NOSEDOWN, ACCELCAL_VEHICLE_POS_NOSEUP, ACCELCAL_VEHICLE_POS_BACK,
    };
    int _apmAccelCalPosIndex = -1;
    bool _apmAccelCalGotAck = false;
    int _apmAccelCalTickCount = 0;

    struct RCChannelOverride
    {
        enum class State
        {
            Ignore,
            Overridden,
            Released
        } state = State::Ignore;
        uint16_t value = 0;
    };

    static constexpr int kRcChannelOverrideChannelCount = 18;
    std::array<RCChannelOverride, kRcChannelOverrideChannelCount> _rcChannelOverrides;

    QMap<MAV_CMD, int> _receivedMavCommandCountMap;
    QMap<MAV_CMD, QMap<int, int>> _receivedMavCommandByCompCountMap;
    QMap<uint32_t, int> _receivedRequestMessageCountMap;
    QMap<int, QMap<int, int>> _receivedRequestMessageByCompAndMsgCountMap;
    QMap<uint32_t, int> _receivedMavlinkMessageCountMap;
    QMap<uint32_t, mavlink_message_t> _lastReceivedMavlinkMessageMap;
    QMap<int, QMap<QString, QVariant>> _mapParamName2Value;
    QMap<int, QMap<QString, MAV_PARAM_TYPE>> _mapParamName2MavParamType;

    struct ADSBVehicle
    {
        QGeoCoordinate coordinate;
        double angle = 0.0;
        double altitude = 0.0;
    };

    QList<ADSBVehicle> _adsbVehicles;
    QList<QGeoCoordinate> _adsbVehicleCoordinates;
    static constexpr int _numberOfVehicles = 5;
    double _adsbAngles[_numberOfVehicles]{};

    static std::atomic<int> _nextVehicleSystemId;

    static constexpr double _defaultVehicleLatitude = 47.397;
    static constexpr double _defaultVehicleLongitude = 8.5455;
    static constexpr double _defaultVehicleHomeAltitude = 488.056;

    static constexpr const char* _failParam = "COM_FLTMODE6";

    static constexpr uint8_t _vehicleComponentId = MAV_COMP_ID_AUTOPILOT1;

    static constexpr uint16_t _logDownloadLogId = 0;
    static constexpr uint32_t _logDownloadFileSize = 1000;

    static constexpr bool _mavlinkStarted = true;

    inline static const QSet<QString> kAPMCalOffsetParams = {
        QStringLiteral("COMPASS_OFS_X"),  QStringLiteral("COMPASS_OFS_Y"),  QStringLiteral("COMPASS_OFS_Z"),
        QStringLiteral("COMPASS_OFS2_X"), QStringLiteral("COMPASS_OFS2_Y"), QStringLiteral("COMPASS_OFS2_Z"),
        QStringLiteral("COMPASS_OFS3_X"), QStringLiteral("COMPASS_OFS3_Y"), QStringLiteral("COMPASS_OFS3_Z"),
        QStringLiteral("INS_ACCOFFS_X"),  QStringLiteral("INS_ACCOFFS_Y"),  QStringLiteral("INS_ACCOFFS_Z"),
    };

    static QList<FlightMode_t> _availableFlightModes;
    static QList<FlightMode_t> _apmCopterAvailableFlightModes;
    static QList<FlightMode_t> _apmPlaneAvailableFlightModes;
    static QList<FlightMode_t> _apmRoverAvailableFlightModes;
    static QList<FlightMode_t> _apmSubAvailableFlightModes;

    const QList<FlightMode_t>& _flightModeList() const;

    std::atomic<bool> _disconnectedEmitted{false};
};
