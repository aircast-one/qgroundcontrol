target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DragToPositionTest.cc
        DragToPositionTest.h
        FlyViewToolBarEditTest.cc
        FlyViewToolBarEditTest.h
        QGCColoredImageTest.cc
        QGCColoredImageTest.h
        OverlayBackdropTest.cc
        OverlayBackdropTest.h
        OverlayPaletteTest.cc
        OverlayPaletteTest.h
        PlatformThemeTest.cc
        PlatformThemeTest.h
        OverlaySegmentedControlTest.cc
        OverlaySegmentedControlTest.h
        OverlayViewSwitchTest.cc
        OverlayViewSwitchTest.h
        OverlayRigTest.cc
        OverlayRigTest.h
        PipRevealTest.cc
        PipRevealTest.h
        PipViewTest.cc
        PipViewTest.h
        PlanToolBarIndicatorsTest.cc
        PlanToolBarIndicatorsTest.h
        PlanViewLayoutTest.cc
        PlanViewLayoutTest.h
        ScreenToolsTest.cc
        ScreenToolsTest.h
        SafeAreaTest.cc
        SafeAreaTest.h
        SettingsSearchTest.cc
        SettingsSearchTest.h
        UnitSystemTest.cc
        UnitSystemTest.h
        VehicleStatusSummaryTest.cc
        VehicleStatusSummaryTest.h
        QuickInteractionTestHelpers.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

# qt_add_qml_module(QmlControlsTest
#     URI qmlcontrolstest
#     VERSION 1.0
#     QML_FILES
#         QGCControlDebug.qml
#         QmlTest.qml
#     IMPORT_PATH ${QT_QML_OUTPUT_DIRECTORY}
# )
