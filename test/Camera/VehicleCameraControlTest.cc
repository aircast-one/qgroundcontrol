#include "VehicleCameraControlTest.h"
#include <QtTest/QSignalSpy>


#include "LinkManager.h"
#include "MavlinkCameraControlInterface.h"
#include "MockConfiguration.h"
#include "MockLink.h"
#include "MultiVehicleManager.h"
#include "QGCCameraManager.h"
#include "Vehicle.h"

void VehicleCameraControlTest::initTestCase()
{
    UnitTest::initTestCase();
    MultiVehicleManager::instance()->init();
}

void VehicleCameraControlTest::init()
{
    UnitTest::init();
    _mockLink = nullptr;
    _vehicle = nullptr;
}

void VehicleCameraControlTest::cleanup()
{
    if (_mockLink) {
        QSignalSpy spyDisconnect(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged);
        _mockLink->disconnect();
        _mockLink = nullptr;

        if (_vehicle) {
            UnitTest::waitForSignal(spyDisconnect, TestTimeout::longMs(), QStringLiteral("activeVehicleChanged"));
        }
        _vehicle = nullptr;

        UnitTest::settleEventLoopForCleanup();
    }

    dumpFailureContextIfTestFailed(QStringLiteral("cleanup"));
    UnitTest::cleanup();
}

void VehicleCameraControlTest::_testCameraCapFlags_data()
{
    // MockConfiguration camera flags
    QTest::addColumn<bool>("captureVideo");
    QTest::addColumn<bool>("captureImage");
    QTest::addColumn<bool>("hasModes");
    QTest::addColumn<bool>("hasVideoStream");
    QTest::addColumn<bool>("canCaptureImageInVideoMode");
    QTest::addColumn<bool>("canCaptureVideoInImageMode");
    QTest::addColumn<bool>("hasBasicZoom");
    QTest::addColumn<bool>("hasTrackingPoint");
    QTest::addColumn<bool>("hasTrackingRectangle");

    // Expected VehicleCameraControl properties
    QTest::addColumn<bool>("expectedCapturesVideo");
    QTest::addColumn<bool>("expectedCapturesPhotos");
    QTest::addColumn<bool>("expectedHasModes");
    QTest::addColumn<bool>("expectedHasZoom");
    QTest::addColumn<bool>("expectedHasVideoStream");
    QTest::addColumn<bool>("expectedPhotosInVideoMode");
    QTest::addColumn<bool>("expectedVideoInPhotoMode");
    QTest::addColumn<bool>("expectedHasTracking");

    // Note: xCapVid and xCapImg (capturesVideo/capturesPhotos) are "virtual" capabilities that account for
    // local gstreamer support. When a camera has a video stream (HAS_VIDEO_STREAM), QGC can locally record
    // video via gstreamer and screen-grab photos from the stream, even if the camera itself doesn't support
    // native video recording or photo capture. See "video-stream-only" and "stream-no-native-capture" rows.

    //                                       capVid   capImg   modes    stream   imgInVid vidInImg zoom     trkPt    trkRect   xCapVid  xCapImg  xModes   xZoom    xStream  xImgVid  xVidImg  xTrack
    QTest::newRow("all-caps")                << true  << true  << true  << true  << true  << true  << true  << true  << true   << true  << true  << true  << true  << true  << true  << true  << true;
    QTest::newRow("no-caps")                 << false << false << false << false << false << false << false << false << false  << false << false << false << false << false << false << false << false;
    QTest::newRow("photo-only")              << false << true  << false << false << false << false << false << false << false  << false << true  << false << false << false << false << false << false;
    QTest::newRow("video-only")              << true  << false << false << false << false << false << false << false << false  << true  << false << false << false << false << false << false << false;
    QTest::newRow("video-stream-only")       << false << false << false << true  << false << false << false << false << false  << true  << true  << false << false << true  << false << false << false;
    QTest::newRow("modes-zoom")              << false << true  << true  << false << false << false << true  << false << false  << false << true  << true  << true  << false << false << false << false;
    QTest::newRow("tracking-point")          << false << true  << false << false << false << false << false << true  << false  << false << true  << false << false << false << false << false << true;
    QTest::newRow("tracking-rect")           << false << true  << false << false << false << false << false << false << true   << false << true  << false << false << false << false << false << true;
    QTest::newRow("tracking-both")           << false << true  << false << false << false << false << false << true  << true   << false << true  << false << false << false << false << false << true;
    QTest::newRow("image-in-video")          << true  << true  << true  << false << true  << false << false << false << false  << true  << true  << true  << false << false << true  << false << false;
    QTest::newRow("video-in-image")          << true  << true  << true  << false << false << true  << false << false << false  << true  << true  << true  << false << false << false << true  << false;
    QTest::newRow("stream-no-native-capture")<< false << false << false << true  << false << false << false << false << false  << true  << true  << false << false << true  << false << false << false;
    QTest::newRow("stream-plus-photo")       << false << true  << false << true  << false << false << false << false << false  << true  << true  << false << false << true  << false << false << false;
}

void VehicleCameraControlTest::_testCameraCapFlags()
{
    // Fetch test data
    QFETCH(bool, captureVideo);
    QFETCH(bool, captureImage);
    QFETCH(bool, hasModes);
    QFETCH(bool, hasVideoStream);
    QFETCH(bool, canCaptureImageInVideoMode);
    QFETCH(bool, canCaptureVideoInImageMode);
    QFETCH(bool, hasBasicZoom);
    QFETCH(bool, hasTrackingPoint);
    QFETCH(bool, hasTrackingRectangle);

    QFETCH(bool, expectedCapturesVideo);
    QFETCH(bool, expectedCapturesPhotos);
    QFETCH(bool, expectedHasModes);
    QFETCH(bool, expectedHasZoom);
    QFETCH(bool, expectedHasVideoStream);
    QFETCH(bool, expectedPhotosInVideoMode);
    QFETCH(bool, expectedVideoInPhotoMode);
    QFETCH(bool, expectedHasTracking);

    // Create MockConfiguration with camera enabled and custom flags
    auto* mockConfig = new MockConfiguration(QStringLiteral("CameraCapFlagsTest"));
    mockConfig->setFirmwareType(MAV_AUTOPILOT_PX4);
    mockConfig->setVehicleType(MAV_TYPE_QUADROTOR);
    mockConfig->setDynamic(true);
    mockConfig->setEnableCamera(true);
    mockConfig->setCameraCaptureVideo(captureVideo);
    mockConfig->setCameraCaptureImage(captureImage);
    mockConfig->setCameraHasModes(hasModes);
    mockConfig->setCameraHasVideoStream(hasVideoStream);
    mockConfig->setCameraCanCaptureImageInVideoMode(canCaptureImageInVideoMode);
    mockConfig->setCameraCanCaptureVideoInImageMode(canCaptureVideoInImageMode);
    mockConfig->setCameraHasBasicZoom(hasBasicZoom);
    mockConfig->setCameraHasTrackingPoint(hasTrackingPoint);
    mockConfig->setCameraHasTrackingRectangle(hasTrackingRectangle);

    // Connect MockLink
    QSignalSpy spyVehicle(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged);
    QVERIFY(spyVehicle.isValid());

    SharedLinkConfigurationPtr linkConfig = LinkManager::instance()->addConfiguration(mockConfig);
    QVERIFY(LinkManager::instance()->createConnectedLink(linkConfig));

    QVERIFY2(UnitTest::waitForSignal(spyVehicle, TestTimeout::longMs(), QStringLiteral("activeVehicleChanged")),
             "Timeout waiting for vehicle connection");

    _vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(_vehicle);

    _mockLink = qobject_cast<MockLink*>(linkConfig->link());
    QVERIFY(_mockLink);

    // Wait for initial connect sequence to complete
    if (!_vehicle->isInitialConnectComplete()) {
        QSignalSpy spyConnect(_vehicle, &Vehicle::initialConnectComplete);
        QVERIFY(spyConnect.isValid());
        QVERIFY2(UnitTest::waitForSignal(spyConnect, TestTimeout::longMs(), QStringLiteral("initialConnectComplete")),
                 "Timeout waiting for initial connect");
    }

    // Wait for camera manager to discover cameras
    QGCCameraManager* cameraManager = _vehicle->cameraManager();
    QVERIFY(cameraManager);

    // MockLinkCamera creates two cameras; wait for both to be discovered
    QVERIFY_TRUE_WAIT(cameraManager->cameras()->count() >= 2, TestTimeout::longMs());

    // Find Camera 1 (MAV_COMP_ID_CAMERA) which has our configured flags.
    // Camera 2 (MAV_COMP_ID_CAMERA2) is always photo-only and not what we're testing.
    MavlinkCameraControlInterface* camera = nullptr;
    for (int i = 0; i < cameraManager->cameras()->count(); i++) {
        auto* cam = qobject_cast<MavlinkCameraControlInterface*>(cameraManager->cameras()->get(i));
        if (cam && cam->compID() == MAV_COMP_ID_CAMERA) {
            camera = cam;
            break;
        }
    }
    QVERIFY2(camera, "Camera 1 (MAV_COMP_ID_CAMERA) not found in camera list");

    // Verify capability properties match expected values
    QCOMPARE(camera->capturesVideo(),     expectedCapturesVideo);
    QCOMPARE(camera->capturesPhotos(),    expectedCapturesPhotos);
    QCOMPARE(camera->hasModes(),          expectedHasModes);
    QCOMPARE(camera->hasZoom(),           expectedHasZoom);
    QCOMPARE(camera->hasVideoStream(),    expectedHasVideoStream);
    QCOMPARE(camera->photosInVideoMode(), expectedPhotosInVideoMode);
    QCOMPARE(camera->videoInPhotoMode(),  expectedVideoInPhotoMode);
    QCOMPARE(camera->hasTracking(),       expectedHasTracking);

    // hasFocus is always false since MockLinkCamera doesn't support CAMERA_CAP_FLAGS_HAS_BASIC_FOCUS
    QCOMPARE(camera->hasFocus(), false);
}

void VehicleCameraControlTest::_testZoomTriggersCameraSettingsRequest()
{
    // A spec-minimal camera does not broadcast CAMERA_SETTINGS after a zoom change,
    // so QGC must re-request it itself once a zoom command is accepted. Otherwise
    // the zoom level (and FOV) shown by QGC goes stale after the first zoom.

    auto* mockConfig = new MockConfiguration(QStringLiteral("CameraZoomSettingsTest"));
    mockConfig->setFirmwareType(MAV_AUTOPILOT_PX4);
    mockConfig->setVehicleType(MAV_TYPE_QUADROTOR);
    mockConfig->setDynamic(true);
    mockConfig->setEnableCamera(true);
    mockConfig->setCameraCaptureImage(true);
    mockConfig->setCameraHasBasicZoom(true);

    QSignalSpy spyVehicle(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged);
    QVERIFY(spyVehicle.isValid());

    SharedLinkConfigurationPtr linkConfig = LinkManager::instance()->addConfiguration(mockConfig);
    QVERIFY(LinkManager::instance()->createConnectedLink(linkConfig));

    QVERIFY2(UnitTest::waitForSignal(spyVehicle, TestTimeout::longMs(), QStringLiteral("activeVehicleChanged")),
             "Timeout waiting for vehicle connection");

    _vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(_vehicle);

    _mockLink = qobject_cast<MockLink*>(linkConfig->link());
    QVERIFY(_mockLink);

    if (!_vehicle->isInitialConnectComplete()) {
        QSignalSpy spyConnect(_vehicle, &Vehicle::initialConnectComplete);
        QVERIFY(spyConnect.isValid());
        QVERIFY2(UnitTest::waitForSignal(spyConnect, TestTimeout::longMs(), QStringLiteral("initialConnectComplete")),
                 "Timeout waiting for initial connect");
    }

    QGCCameraManager* cameraManager = _vehicle->cameraManager();
    QVERIFY(cameraManager);
    QVERIFY_TRUE_WAIT(cameraManager->cameras()->count() >= 2, TestTimeout::longMs());

    MavlinkCameraControlInterface* camera = nullptr;
    for (int i = 0; i < cameraManager->cameras()->count(); i++) {
        auto* cam = qobject_cast<MavlinkCameraControlInterface*>(cameraManager->cameras()->get(i));
        if (cam && cam->compID() == MAV_COMP_ID_CAMERA) {
            camera = cam;
            break;
        }
    }
    QVERIFY2(camera, "Camera 1 (MAV_COMP_ID_CAMERA) not found in camera list");
    QVERIFY(camera->hasZoom());

    // Wait for the initial CAMERA_SETTINGS exchange to settle: the mock reports zoom
    // level 1.0, so once QGC shows it the initial request/retry cycle is complete.
    QVERIFY_TRUE_WAIT(qFuzzyCompare(camera->zoomLevel(), 1.0), TestTimeout::longMs());

    // QGC alternates between REQUEST_MESSAGE and the deprecated REQUEST_CAMERA_SETTINGS
    // on retries (dual-transport migration), so accept either as a settings request.
    auto settingsRequestCount = [this]() {
        return _mockLink->receivedRequestMessageCount(MAV_COMP_ID_CAMERA, MAVLINK_MSG_ID_CAMERA_SETTINGS)
             + _mockLink->receivedMavCommandCount(MAV_CMD_REQUEST_CAMERA_SETTINGS, MAV_COMP_ID_CAMERA);
    };

    // An accepted zoom command must cause QGC to re-request CAMERA_SETTINGS on its own
    const int baselineAfterConnect = settingsRequestCount();
    camera->setZoomLevel(50.0);
    QTRY_VERIFY2_WITH_TIMEOUT(settingsRequestCount() > baselineAfterConnect,
                              "QGC did not re-request CAMERA_SETTINGS after setZoomLevel was accepted",
                              TestTimeout::longMs());
    // And the fresh settings update QGC's stale zoom level
    QVERIFY_TRUE_WAIT(qFuzzyCompare(camera->zoomLevel(), 50.0), TestTimeout::longMs());

    // Continuous zoom (start/stop) must trigger a refresh as well
    const int baselineAfterSetZoom = settingsRequestCount();
    camera->startZoom(1);
    camera->stopZoom();
    QTRY_VERIFY2_WITH_TIMEOUT(settingsRequestCount() > baselineAfterSetZoom,
                              "QGC did not re-request CAMERA_SETTINGS after startZoom/stopZoom were accepted",
                              TestTimeout::longMs());
}

UT_REGISTER_TEST(VehicleCameraControlTest, TestLabel::Integration, TestLabel::Vehicle)

void VehicleCameraControlTest::_testFightModeRefusesEveryCapture()
{
    auto* mockConfig = new MockConfiguration(QStringLiteral("CameraFightModeTest"));
    mockConfig->setFirmwareType(MAV_AUTOPILOT_PX4);
    mockConfig->setVehicleType(MAV_TYPE_QUADROTOR);
    mockConfig->setDynamic(true);
    mockConfig->setEnableCamera(true);
    mockConfig->setCameraCaptureImage(true);
    mockConfig->setCameraCaptureVideo(true);

    QSignalSpy spyVehicle(MultiVehicleManager::instance(), &MultiVehicleManager::activeVehicleChanged);
    QVERIFY(spyVehicle.isValid());

    SharedLinkConfigurationPtr linkConfig = LinkManager::instance()->addConfiguration(mockConfig);
    QVERIFY(LinkManager::instance()->createConnectedLink(linkConfig));
    QVERIFY2(UnitTest::waitForSignal(spyVehicle, TestTimeout::longMs(), QStringLiteral("activeVehicleChanged")),
             "Timeout waiting for vehicle connection");

    _vehicle = MultiVehicleManager::instance()->activeVehicle();
    QVERIFY(_vehicle);
    _mockLink = qobject_cast<MockLink*>(linkConfig->link());
    QVERIFY(_mockLink);

    if (!_vehicle->isInitialConnectComplete()) {
        QSignalSpy spyConnect(_vehicle, &Vehicle::initialConnectComplete);
        QVERIFY(spyConnect.isValid());
        QVERIFY2(UnitTest::waitForSignal(spyConnect, TestTimeout::longMs(), QStringLiteral("initialConnectComplete")),
                 "Timeout waiting for initial connect");
    }

    QGCCameraManager* cameraManager = _vehicle->cameraManager();
    QVERIFY(cameraManager);
    QVERIFY_TRUE_WAIT(cameraManager->cameras()->count() >= 2, TestTimeout::longMs());

    MavlinkCameraControlInterface* camera = nullptr;
    for (int i = 0; i < cameraManager->cameras()->count(); i++) {
        auto* cam = qobject_cast<MavlinkCameraControlInterface*>(cameraManager->cameras()->get(i));
        if (cam && cam->compID() == MAV_COMP_ID_CAMERA) {
            camera = cam;
            break;
        }
    }
    QVERIFY2(camera, "Camera 1 (MAV_COMP_ID_CAMERA) not found in camera list");

    const auto sendNamedValue = [this](uint8_t compId, const char* name, int32_t number) {
        mavlink_named_value_int_t value{};
        strncpy(value.name, name, sizeof(value.name));
        value.value = number;
        mavlink_message_t message{};
        (void) mavlink_msg_named_value_int_encode(static_cast<uint8_t>(_mockLink->vehicleId()), compId, &message,
                                                  &value);
        _mockLink->respondWithMavlinkMessage(message);
    };

    QSignalSpy spyHome(_vehicle, &Vehicle::homePositionChanged);
    QVERIFY(spyHome.isValid());
    sendNamedValue(MAV_COMP_ID_AUTOPILOT1, "FIGHT", 1);
    sendNamedValue(MAV_COMP_ID_ONBOARD_COMPUTER, "OTHER", 1);
    sendNamedValue(MAV_COMP_ID_ONBOARD_COMPUTER, "FIGHT", 0);
    mavlink_home_position_t home{};
    home.latitude = 471234567;
    home.longitude = 85432100;
    home.altitude = 10000;
    home.q[0] = 1.0f;
    mavlink_message_t homeMessage{};
    (void) mavlink_msg_home_position_encode(static_cast<uint8_t>(_mockLink->vehicleId()), MAV_COMP_ID_AUTOPILOT1,
                                            &homeMessage, &home);
    _mockLink->respondWithMavlinkMessage(homeMessage);
    QVERIFY2(UnitTest::waitForSignal(spyHome, TestTimeout::longMs(), QStringLiteral("homePositionChanged")),
             "Timeout waiting for the home position sent after the other values");
    QVERIFY2(!_vehicle->fightMode(), "only aircastd's FIGHT=1 from the onboard computer turns fight mode on");

    sendNamedValue(MAV_COMP_ID_ONBOARD_COMPUTER, "FIGHT", 1);
    QVERIFY_TRUE_WAIT(_vehicle->fightMode(), TestTimeout::longMs());

    _mockLink->clearReceivedMavCommandCounts();
    const QRegularExpression refused(QStringLiteral("Fight mode: nothing is recorded"));
    expectAppMessage(refused);
    QVERIFY(!camera->takePhoto());
    verifyExpectedLogMessage();
    expectAppMessage(refused);
    QVERIFY(!camera->startVideoRecording());
    verifyExpectedLogMessage();
    expectAppMessage(refused);
    QVERIFY(!camera->toggleVideoRecording());
    verifyExpectedLogMessage();
    expectAppMessage(refused);
    _vehicle->triggerSimpleCamera();
    verifyExpectedLogMessage();
    expectLogMessage("Camera.VehicleCameraControl", QtWarningMsg, QRegularExpression(QStringLiteral("already idle")));
    QVERIFY2(camera->stopVideoRecording(), "stopping a recording is never refused");
    verifyExpectedLogMessage();

    _vehicle->sendMavCommand(_vehicle->defaultComponentId(), MAV_CMD_REQUEST_MESSAGE, false,
                             MAVLINK_MSG_ID_HOME_POSITION);
    QTRY_VERIFY_WITH_TIMEOUT(_mockLink->receivedMavCommandCount(MAV_CMD_REQUEST_MESSAGE) > 0, TestTimeout::longMs());
    QCOMPARE(_mockLink->receivedMavCommandCount(MAV_CMD_IMAGE_START_CAPTURE), 0);
    QCOMPARE(_mockLink->receivedMavCommandCount(MAV_CMD_VIDEO_START_CAPTURE), 0);
    QCOMPARE(_mockLink->receivedMavCommandCount(MAV_CMD_DO_DIGICAM_CONTROL), 0);
}
