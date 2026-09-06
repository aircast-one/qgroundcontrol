target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        DragToPositionTest.cc
        DragToPositionTest.h
        OverlayBackdropTest.cc
        OverlayBackdropTest.h
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
        AppSettingsLayoutTest.cc
        AppSettingsLayoutTest.h
        PlanViewLayoutTest.cc
        PlanViewLayoutTest.h
        ScreenToolsTest.cc
        ScreenToolsTest.h
        QuickInteractionTestHelpers.h
        PlatformThemeTest.cc
        PlatformThemeTest.h
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
