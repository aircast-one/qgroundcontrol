/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "UnitTestList.h"
#include "UnitTest.h"
#include "QGCLoggingCategory.h"

// ADSB
#include "ADSBTest.h"

// Bridge
#include "QGCBridgeCoreTest.h"
#ifdef QGC_WFB_ENABLED
#include "PacketRadioTest.h"
#endif

// AnalyzeView
#include "ExifParserTest.h"
// #include "GeoTagControllerTest.h"
// #include "MavlinkLogTest.h"
#include "LogDownloadTest.h"
#include "PX4LogParserTest.h"
// #include "ULogParserTest.h"



// AutoPilotPlugins
// #include "RadioConfigTest.h"

// AircastSetup
#include "AircastDeviceSetupTest.h"

// Camera
#include "QGCCameraManagerTest.h"

// Comms
#include "QGCSerialPortInfoTest.h"
#include "TCPConfigurationTest.h"
#include "TCPLinkErrorTest.h"
#include "LinkStateTest.h"
#include "UDPConfigurationTest.h"

// DebugApi
#include "DebugApiServerTest.h"

// FactSystem
#include "FactSystemTestGeneric.h"
#include "FactSystemTestPX4.h"
#include "ParameterManagerTest.h"

// FollowMe
#include "FollowMeTest.h"

// GPS
#include "GpsTest.h"

// MAVLink
#include "StatusTextHandlerTest.h"
#include "SigningTest.h"
#include "SysStatusSensorInfoTest.h"

// MissionManager
#include "CameraCalcTest.h"
#include "CameraSectionTest.h"
#include "CorridorScanComplexItemTest.h"
// #include "FWLandingPatternTest.h"
// #include "LandingComplexItemTest.h"
// #include "MissionCommandTreeEditorTest.h"
#include "MissionCommandTreeTest.h"
#include "MissionControllerManagerTest.h"
#include "MissionControllerTest.h"
#include "MissionItemTest.h"
#include "MissionManagerTest.h"
#include "MissionSettingsTest.h"
#include "PlanMasterControllerTest.h"
#include "QGCMapPolygonTest.h"
#include "QGCMapPolylineTest.h"
// #include "SectionTest.h"
#include "SimpleMissionItemTest.h"
#include "SpeedSectionTest.h"
#include "StructureScanComplexItemTest.h"
#include "SurveyComplexItemTest.h"
#include "TransectStyleComplexItemTest.h"
// #include "VisualMissionItemTest.h"

// qgcunittest
#include "ComponentInformationCacheTest.h"
#include "ComponentInformationTranslationTest.h"

// QmlControls
#include "DragToPositionTest.h"
#include "OverlayBackdropTest.h"
#include "OverlayPaletteTest.h"
#include "PlatformThemeTest.h"
#include "QGCColoredImageTest.h"
#include "OverlaySegmentedControlTest.h"
#include "FlyViewToolBarEditTest.h"
#include "OverlayViewSwitchTest.h"
#include "PlanToolBarIndicatorsTest.h"
#include "PlanViewLayoutTest.h"
#include "ScreenToolsTest.h"
#include "SafeAreaTest.h"
#include "SettingsSearchTest.h"
#include "UnitSystemTest.h"
#include "VehicleStatusSummaryTest.h"
#include "OverlayRigTest.h"
#include "PipRevealTest.h"
#include "PipViewTest.h"
#include "VideoTileTest.h"

// VideoManager
#include "VideoManagerTest.h"

// Terrain
#include "TerrainQueryTest.h"
#include "TerrainTileTest.h"

// QtLocationPlugin
#include "QGCTileCacheWorkerTest.h"
#include "QGeoMapReplyQGCTest.h"

// UI

// Utilities
// Audio
#include "AudioOutputTest.h"
// Compression
#include "DecompressionTest.h"
// FileSystem
#include "QGCFileDownloadTest.h"
// Geo
#include "GeoTest.h"
// Shape
#include "ShapeTest.h"

// Vehicle
// Components
#include "ComponentInformationCacheTest.h"
#include "ComponentInformationTranslationTest.h"
#include "FTPManagerTest.h"
// #include "InitialConnectTest.h"
#include "MAVLinkLogManagerTest.h"
// #include "RequestMessageTest.h"
// #include "SendMavCommandWithHandlerTest.h"
// #include "SendMavCommandWithSignalingTest.h"
#include "RcChannelOverrideTest.h"
#include "CameraControlTest.h"
#include "FlightMapTest.h"
#include "TelemetryChipsTest.h"
#include "TerrainProgressTest.h"
#include "FirmwareUpdateAvailableTest.h"
#include "FlightModeReportingTest.h"
#include "VehicleLinkManagerTest.h"

// Missing
// #include "FlightGearUnitTest.h"
// #include "LinkManagerTest.h"
// #include "SendMavCommandTest.h"
// #include "TCPLinkTest.h"

QGC_LOGGING_CATEGORY(UnitTestsLog, "qgc.test.unittestlist")

int runTests(bool stress, QStringView unitTestOptions)
{
    // ADSB
    UT_REGISTER_TEST(ADSBTest)
    UT_REGISTER_TEST(QGCBridgeCoreTest)
#ifdef QGC_WFB_ENABLED
    UT_REGISTER_TEST(PacketRadioTest)
#endif

    // AnalyzeView
    UT_REGISTER_TEST(ExifParserTest)
    // UT_REGISTER_TEST(GeoTagControllerTest)
    // UT_REGISTER_TEST(MavlinkLogTest)
    UT_REGISTER_TEST(LogDownloadTest)
    UT_REGISTER_TEST(PX4LogParserTest)
    // UT_REGISTER_TEST(ULogParserTest)

    // AutoPilotPlugins
    // UT_REGISTER_TEST(RadioConfigTest)

    // AircastSetup
    UT_REGISTER_TEST(AircastDeviceSetupTest)

    // Camera
    UT_REGISTER_TEST(QGCCameraManagerTest)

    // Comms
    UT_REGISTER_TEST(QGCSerialPortInfoTest)
    UT_REGISTER_TEST(TCPConfigurationTest)
    UT_REGISTER_TEST(TCPLinkErrorTest)
    UT_REGISTER_TEST(LinkStateTest)
    UT_REGISTER_TEST(UDPConfigurationTest)

    // DebugApi
    UT_REGISTER_TEST(DebugApiServerTest)

    // FactSystem
    UT_REGISTER_TEST(FactSystemTestGeneric)
    UT_REGISTER_TEST(FactSystemTestPX4)
    UT_REGISTER_TEST(ParameterManagerTest)

    // FollowMe
    UT_REGISTER_TEST(FollowMeTest)

    // GPS
    // UT_REGISTER_TEST(GpsTest)

    // MAVLink
    UT_REGISTER_TEST(StatusTextHandlerTest)
    UT_REGISTER_TEST(SigningTest)
    UT_REGISTER_TEST(SysStatusSensorInfoTest)

    // MissionManager
    UT_REGISTER_TEST(CameraCalcTest)
    UT_REGISTER_TEST(CameraSectionTest)
    UT_REGISTER_TEST(CorridorScanComplexItemTest)
    // UT_REGISTER_TEST(FWLandingPatternTest)
    // UT_REGISTER_TEST(LandingComplexItemTest)
    // UT_REGISTER_TEST_STANDALONE(MissionCommandTreeEditorTest)
    UT_REGISTER_TEST(MissionCommandTreeTest)
    UT_REGISTER_TEST(MissionControllerManagerTest)
    UT_REGISTER_TEST(MissionControllerTest)
    UT_REGISTER_TEST(MissionItemTest)
    UT_REGISTER_TEST(MissionManagerTest)
    UT_REGISTER_TEST(MissionSettingsTest)
    UT_REGISTER_TEST(PlanMasterControllerTest)
    UT_REGISTER_TEST(QGCMapPolygonTest)
    UT_REGISTER_TEST(QGCMapPolylineTest)
    // UT_REGISTER_TEST(SectionTest)
    UT_REGISTER_TEST(SimpleMissionItemTest)
    UT_REGISTER_TEST(SpeedSectionTest)
    UT_REGISTER_TEST(StructureScanComplexItemTest)
    UT_REGISTER_TEST(SurveyComplexItemTest)
    UT_REGISTER_TEST(TransectStyleComplexItemTest)
    // UT_REGISTER_TEST(VisualMissionItemTest)

    // qgcunittest

    // QmlControls
    UT_REGISTER_TEST(DragToPositionTest)
    UT_REGISTER_TEST(OverlayBackdropTest)
    UT_REGISTER_TEST(OverlayPaletteTest)
    UT_REGISTER_TEST(PlatformThemeTest)
    UT_REGISTER_TEST(QGCColoredImageTest)
    UT_REGISTER_TEST(OverlaySegmentedControlTest)
    UT_REGISTER_TEST(FlyViewToolBarEditTest)
    UT_REGISTER_TEST(OverlayViewSwitchTest)
    UT_REGISTER_TEST(PlanToolBarIndicatorsTest)
    UT_REGISTER_TEST(PlanViewLayoutTest)
    UT_REGISTER_TEST(ScreenToolsTest)
    UT_REGISTER_TEST(SafeAreaTest)
    UT_REGISTER_TEST(SettingsSearchTest)
    UT_REGISTER_TEST(UnitSystemTest)
    UT_REGISTER_TEST(VehicleStatusSummaryTest)
    UT_REGISTER_TEST(OverlayRigTest)
    UT_REGISTER_TEST(PipRevealTest)
    UT_REGISTER_TEST(PipViewTest)
    UT_REGISTER_TEST(VideoTileTest)

    // VideoManager
    UT_REGISTER_TEST(VideoManagerTest)

    // Terrain
    UT_REGISTER_TEST(TerrainQueryTest)
    UT_REGISTER_TEST(TerrainTileTest)

    // QtLocationPlugin
    UT_REGISTER_TEST(QGCTileCacheWorkerTest)
    UT_REGISTER_TEST(QGeoMapReplyQGCTest)

    // UI

    // Utilities
    // Audio
    UT_REGISTER_TEST(AudioOutputTest)
    // Compression
    UT_REGISTER_TEST(DecompressionTest)
    // FileSystem
    UT_REGISTER_TEST(QGCFileDownloadTest)
    // Geo
    UT_REGISTER_TEST(GeoTest)
    // Shape
    UT_REGISTER_TEST(ShapeTest)

    // Vehicle
    // Components
    UT_REGISTER_TEST(ComponentInformationCacheTest)
    UT_REGISTER_TEST(ComponentInformationTranslationTest)
    UT_REGISTER_TEST(FTPManagerTest)
    // UT_REGISTER_TEST(InitialConnectTest)
    UT_REGISTER_TEST(MAVLinkLogManagerTest)
    // UT_REGISTER_TEST(RequestMessageTest)
    // UT_REGISTER_TEST(SendMavCommandWithHandlerTest)
    // UT_REGISTER_TEST(SendMavCommandWithSignalingTest)
    UT_REGISTER_TEST(RcChannelOverrideTest)
    UT_REGISTER_TEST(CameraControlTest)
    UT_REGISTER_TEST(FlightMapTest)
    UT_REGISTER_TEST(TelemetryChipsTest)
    UT_REGISTER_TEST(TerrainProgressTest)
    UT_REGISTER_TEST(FirmwareUpdateAvailableTest)
    UT_REGISTER_TEST(FlightModeReportingTest)
    UT_REGISTER_TEST(VehicleLinkManagerTest)

    // Missing
    // UT_REGISTER_TEST(FlightGearUnitTest)
    // UT_REGISTER_TEST(LinkManagerTest)
    // UT_REGISTER_TEST(SendMavCommandTest)
    // UT_REGISTER_TEST(TCPLinkTest)

    int result = 0;

    for (int i=0; i < (stress ? 20 : 1); i++) {
        // Run the test
        const int failures = UnitTest::run(unitTestOptions);
        if (failures == 0) {
            qDebug() << "ALL TESTS PASSED";
            result = 0;
        } else {
            qDebug() << failures << " TESTS FAILED!";
            result = -failures;
            break;
        }
    }

    return result;
}
