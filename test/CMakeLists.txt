
if(NOT QGC_BUILD_TESTING)
    return()
endif()

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        UnitTestList.cc
        UnitTestList.h
)

find_package(Qt6 REQUIRED COMPONENTS Test)
target_link_libraries(${CMAKE_PROJECT_NAME} PRIVATE Qt6::Test)
set_target_properties(${CMAKE_PROJECT_NAME} PROPERTIES QT_SKIP_DEFAULT_TESTCASE_DIRS TRUE)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

target_precompile_headers(${CMAKE_PROJECT_NAME} PRIVATE
    <QtTest/QTest>
    <UnitTest.h>
    <MultiSignalSpy.h>
)

include(QGCTest)
include(Coverage)
include(Sanitizers)

if(QGC_BUILD_TEST_BINARY)
    message(WARNING
        "QGC_BUILD_TEST_BINARY=ON: qgc_tests target wiring is not yet implemented; "
        "tests will continue to run from the main app binary. See test/CMakeLists.txt.")
endif()

if(QGC_ENABLE_ASAN OR QGC_ENABLE_UBSAN OR QGC_ENABLE_TSAN OR QGC_ENABLE_MSAN)
    set(QGC_TEST_TIMEOUT_UNIT 180 CACHE STRING "Timeout for unit tests (seconds)" FORCE)
    set(QGC_TEST_TIMEOUT_INTEGRATION 360 CACHE STRING "Timeout for integration tests (seconds)" FORCE)
    set(QGC_TEST_TIMEOUT_SLOW 540 CACHE STRING "Timeout for slow tests (seconds)" FORCE)
    set(QGC_TEST_TIMEOUT_DEFAULT 270 CACHE STRING "Default test timeout (seconds)" FORCE)
    set(QGC_TEST_TIMEOUT_EXTENDED 360)
else()
    set(QGC_TEST_TIMEOUT_EXTENDED 180)
endif()

add_subdirectory(UnitTestFramework)
add_subdirectory(ADSB)
add_subdirectory(Bridge)
if(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    add_subdirectory(PacketRadio)
    add_qgc_test(PacketRadioTest LABELS Unit)
endif()
add_subdirectory(AircastSetup)
add_qgc_test(AircastDeviceSetupTest LABELS Unit)
add_subdirectory(DebugApi)
add_qgc_test(DebugApiServerTest LABELS Unit)
add_subdirectory(FlightDisplay)
add_qgc_test(VideoTileTest LABELS Unit)
add_subdirectory(AnalyzeView)
add_subdirectory(AutoPilotPlugins)
add_subdirectory(Camera)
add_subdirectory(VideoManager)
add_subdirectory(Comms)
add_qgc_test(SkydroidH16LinksTest LABELS Unit Comms)
add_subdirectory(FactSystem)
add_subdirectory(FlyView)
add_subdirectory(FollowMe)
add_subdirectory(GPS)
add_subdirectory(Joystick)
add_subdirectory(MAVLink)
add_subdirectory(MissionManager)
add_subdirectory(PositionManager)
add_subdirectory(QtLocationPlugin)
add_subdirectory(QmlControls)
add_subdirectory(Settings)
add_subdirectory(Terrain)
add_subdirectory(Utilities)
add_subdirectory(Viewer3D)
add_subdirectory(Vehicle)
add_subdirectory(QmlUITests)

set(_qgc_test_qml
    Comms/UdpSettingsTest.qml
    DebugApi/DebugApiTest.qml
    FlightDisplay/CameraControlLayerTest.qml
    FlightDisplay/CameraControlSettingsTest.qml
    FlightDisplay/FlightMapWheelTest.qml
    FlightDisplay/RcOverrideIndicatorTest.qml
    FlightDisplay/StatusPillTest.qml
    FlightDisplay/TelemetryChipsTest.qml
    FlightDisplay/TerrainProgressTest.qml
    FlightDisplay/VideoTileTest.qml
    MissionManager/MissionCommandTreeEditorTestWindow.qml
    QmlControls/DragToPositionTest.qml
    QmlControls/FlyViewToolBarEditTest.qml
    QmlControls/OverlayRigTest.qml
    QmlControls/OverlaySegmentedControlTest.qml
    QmlControls/OverlayViewSwitchTest.qml
    QmlControls/PipViewTest.qml
    QmlControls/PlanToolBarIndicatorsTest.qml
    QmlControls/PlanViewLayoutTest.qml
    QmlControls/QGCColoredImageTest.qml
    QmlControls/SafeAreaTest.qml
    QmlControls/ScreenToolsTest.qml
    QmlControls/SettingsSearchTest.qml
    QmlControls/VehicleStatusSummaryTest.qml
)
foreach(_qml IN LISTS _qgc_test_qml)
    cmake_path(GET _qml FILENAME _alias)
    set_source_files_properties(${_qml} PROPERTIES QT_RESOURCE_ALIAS ${_alias})
endforeach()
qt_add_resources(${CMAKE_PROJECT_NAME} "UnitTestQml_res"
    PREFIX "/unittest"
    FILES ${_qgc_test_qml}
)
