if(NOT MACOS)
    return()
endif()

option(QGC_NATIVE_UI "Build the in-tree Swift UI into the macOS app" ON)

list(LENGTH CMAKE_OSX_ARCHITECTURES QGC_SWIFT_ARCH_COUNT)
if(QGC_NATIVE_UI AND QGC_SWIFT_ARCH_COUNT GREATER 1)
    message(STATUS "macos/: Swift cannot target ${CMAKE_OSX_ARCHITECTURES} in one library; building the universal app without the in-tree native UI")
    set(QGC_NATIVE_UI OFF)
endif()

if(NOT QGC_NATIVE_UI)
    return()
endif()

enable_language(Swift)

set_directory_properties(PROPERTIES COMPILE_DEFINITIONS "")

add_library(QGCNativeUI STATIC
    Sources/AnalyzeWindow.swift
    Sources/LogDownload.swift
    Sources/LogEntryModel.swift
    Sources/Vibration.swift
    Sources/VibrationReading.swift
    Sources/ParameterModel.swift
    Sources/ParameterRow.swift
    Sources/Parameters.swift
    Sources/VehicleSetupWindow.swift
    Sources/VehicleComponentModel.swift
    Sources/VehicleComponents.swift
    Sources/FlightModePositions.swift
    Sources/PageSelection.swift
    Sources/SafetySections.swift
    Sources/SensorHealth.swift
    Sources/CalibrationModel.swift
    Sources/RadioModel.swift
    Sources/VideoModel.swift
    Sources/VideoSourceModel.swift
    Sources/CameraControlModel.swift
    Sources/Video.swift
    Sources/VideoSurface.swift
    Sources/Radio.swift
    Sources/Sensors.swift
    Sources/CachedTileOverlay.swift
    Sources/TilePyramid.swift
    Sources/MissionMap.swift
    Sources/VehicleMarkerModel.swift
    Sources/MapFraming.swift
    Sources/Mission.swift
    Sources/FenceRally.swift
    Sources/FenceRallyModel.swift
    Sources/MissionItemModel.swift
    Sources/MissionCommandModel.swift
    Sources/MissionItemKind.swift
    Sources/ItemFactModel.swift
    Sources/CameraChoiceModel.swift
    Sources/AltitudeModeModel.swift
    Sources/TerrainProfileModel.swift
    Sources/PlanWindow.swift
    Sources/PlanSummaryModel.swift
    Sources/WriteReportModel.swift
    Sources/ItemSpeedModel.swift
    Sources/LaunchPositionModel.swift
    Sources/MeasureModel.swift
    Sources/Motors.swift
    Sources/RemoteSupport.swift
    Sources/SetupPageModel.swift
    Sources/MapClick.swift
    Sources/MapClickModel.swift
    Sources/MapCentreModel.swift
    Sources/FlyOverlayModel.swift
    Sources/MapFollowModel.swift
    Sources/MapScaleModel.swift
    Sources/TerrainDownloadModel.swift
    Sources/MotorTestModel.swift
    Sources/RemoteSupportModel.swift
    Sources/AppUnits.swift
    Sources/VehicleSetupTextModel.swift
    Sources/SurveyStatsModel.swift
    Sources/MissionVehicleModel.swift
    Sources/GeoTagModel.swift
    Sources/GeoTag.swift
    Sources/MavlinkInspectorModel.swift
    Sources/MavlinkInspector.swift
    Sources/BatteryReadingModel.swift
    Sources/Power.swift
    Sources/FrameSetupModel.swift
    Sources/VehicleMessageModel.swift
    Sources/PreflightModel.swift
    Sources/VehicleWarningModel.swift
    Sources/InstrumentModel.swift
    Sources/Instruments.swift
    Sources/GuidedActionsModel.swift
    Sources/GuidedValueModel.swift
    Sources/GuidedActions.swift
    Sources/Frame.swift
    Sources/OverlayKit.swift
    Sources/AppShell.swift
    Sources/Bridge.swift
    Sources/Fact.swift
    Sources/SettingsPages.swift
    Sources/AboutModel.swift
    Sources/SettingsStore.swift
    Sources/SettingsWindow.swift
    Sources/ConnectionsSection.swift
    Sources/Controls.swift
    Sources/LinkConfigModel.swift
    Sources/Links.swift
    Sources/NativeDebug.swift
    Sources/NativeProbe.swift
    Sources/Probeable.swift
    Sources/QtHostWindow.swift
    Sources/FlyTelemetryModel.swift
    Sources/FlyDetailModel.swift
    Sources/FlightModeModel.swift
    Sources/Fly.swift
    Sources/FlyWindow.swift
)

set_target_properties(QGCNativeUI PROPERTIES
    AUTOMOC OFF
    AUTOUIC OFF
    AUTORCC OFF
)

# CMake does not pass the deployment target or architecture to swiftc, so the Swift
# objects claim the host OS and the linker warns that they outrun the app's 12.0
# floor. One -target per architecture is all swiftc accepts, so a universal build
# needs one Swift library per arch lipo'd together - not wired up yet.
if(CMAKE_OSX_ARCHITECTURES)
    set(QGC_SWIFT_ARCH ${CMAKE_OSX_ARCHITECTURES})
else()
    execute_process(COMMAND uname -m OUTPUT_VARIABLE QGC_SWIFT_ARCH OUTPUT_STRIP_TRAILING_WHITESPACE)
endif()

target_compile_options(QGCNativeUI PRIVATE
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-target ${QGC_SWIFT_ARCH}-apple-macosx${CMAKE_OSX_DEPLOYMENT_TARGET}>"
)

set(QGC_BRIDGE_DIR ${CMAKE_SOURCE_DIR}/src/Bridge)

target_compile_options(QGCNativeUI PRIVATE
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-Xcc -fmodule-map-file=${QGC_BRIDGE_DIR}/module.modulemap>"
    "$<$<COMPILE_LANGUAGE:Swift>:SHELL:-Xcc -I${QGC_BRIDGE_DIR}>"
)

# Targeting the app's deployment floor makes swiftc emit back-deployment shims
# (libswiftCompatibility*). The Swift driver would find them; clang++ drives this
# link, so point it at the toolchain's static Swift libraries.
execute_process(
    COMMAND xcrun --find swiftc
    OUTPUT_VARIABLE QGC_SWIFTC_PATH
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
get_filename_component(QGC_SWIFT_TOOLCHAIN "${QGC_SWIFTC_PATH}" DIRECTORY)
get_filename_component(QGC_SWIFT_TOOLCHAIN "${QGC_SWIFT_TOOLCHAIN}" DIRECTORY)

target_link_directories(${QGC_APP_TARGET} PRIVATE "${QGC_SWIFT_TOOLCHAIN}/lib/swift/macosx")

target_include_directories(${QGC_APP_TARGET} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_compile_definitions(${QGC_APP_TARGET} PRIVATE QGC_NATIVE_UI)

target_link_libraries(${QGC_APP_TARGET}
    PRIVATE
        QGCNativeUI
        "-framework AppKit"
        "-framework SwiftUI"
        "-framework MapKit"
)
