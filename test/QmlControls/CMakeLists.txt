# ============================================================================
# QmlControls Unit Tests
# ============================================================================

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        ObjectItemModelBaseTest.cc
        ObjectItemModelBaseTest.h
        ObjectListModelBaseTest.cc
        ObjectListModelBaseTest.h
        QmlObjectListModelTest.cc
        QmlObjectListModelTest.h
        QmlObjectTreeModelTest.cc
        QmlObjectTreeModelTest.h
        TestDirtyObject.h
        DragToPositionTest.cc
        DragToPositionTest.h
        FlyViewToolBarEditTest.cc
        FlyViewToolBarEditTest.h
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
        ColoredSvgImageProviderTest.cc
        ColoredSvgImageProviderTest.h
        QuickInteractionTestHelpers.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

add_qgc_test(ObjectItemModelBaseTest LABELS Unit)
add_qgc_test(ObjectListModelBaseTest LABELS Unit)
add_qgc_test(QmlObjectListModelTest LABELS Unit)
add_qgc_test(QmlObjectTreeModelTest LABELS Unit)
add_qgc_test(DragToPositionTest LABELS Unit)
add_qgc_test(PipViewTest LABELS Unit)
add_qgc_test(ColoredSvgImageProviderTest LABELS Unit)
add_qgc_test(FlyViewToolBarEditTest LABELS Unit)
add_qgc_test(OverlayBackdropTest LABELS Unit)
add_qgc_test(OverlayPaletteTest LABELS Unit)
add_qgc_test(OverlayRigTest LABELS Unit)
add_qgc_test(OverlaySegmentedControlTest LABELS Unit)
add_qgc_test(OverlayViewSwitchTest LABELS Unit)
add_qgc_test(PipRevealTest LABELS Unit)
add_qgc_test(PlanToolBarIndicatorsTest LABELS Unit)
add_qgc_test(PlanViewLayoutTest LABELS Unit)
add_qgc_test(PlatformThemeTest LABELS Unit)
add_qgc_test(SafeAreaTest LABELS Unit)
add_qgc_test(ScreenToolsTest LABELS Unit)
add_qgc_test(SettingsSearchTest LABELS Unit)
add_qgc_test(UnitSystemTest LABELS Unit)
add_qgc_test(VehicleStatusSummaryTest LABELS Unit)
