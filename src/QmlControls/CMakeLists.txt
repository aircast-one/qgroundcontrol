target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        EditPositionDialogController.cc
        EditPositionDialogController.h
        FactValueGrid.cc
        FactValueGrid.h
        FlightPathSegment.cc
        FlightPathSegment.h
        HorizontalFactValueGrid.cc
        HorizontalFactValueGrid.h
        InstrumentValueData.cc
        InstrumentValueData.h
        MavlinkAction.cc
        MavlinkAction.h
        MavlinkActionManager.cc
        MavlinkActionManager.h
        OverlayPhysics.cc
        OverlayPhysics.h
        ParameterEditorController.cc
        ParameterEditorController.h
        QGCFenceCircle.cc
        QGCFenceCircle.h
        QGCFencePolygon.cc
        QGCFencePolygon.h
        QGCFileDialogController.cc
        QGCFileDialogController.h
        QGCGeoBoundingCube.cc
        QGCGeoBoundingCube.h
        QGCImageProvider.cc
        QGCImageProvider.h
        QGCMapCircle.cc
        QGCMapCircle.h
        QGCMapPolygon.cc
        QGCMapPolygon.h
        QGCMapPolyline.cc
        QGCMapPolyline.h
        QGCPalette.cc
        QGCPalette.h
        QGCQGeoCoordinate.cc
        QGCQGeoCoordinate.h
        QGroundControlQmlGlobal.cc
        QGroundControlQmlGlobal.h
        QmlObjectListModel.cc
        QmlObjectListModel.h
        QmlUnitsConversion.h
        RCChannelMonitorController.cc
        RCChannelMonitorController.h
        RCToParamDialogController.cc
        RCToParamDialogController.h
        ScreenToolsController.cc
        ScreenToolsController.h
        TerrainProfile.cc
        TerrainProfile.h
        ToolStripAction.cc
        ToolStripAction.h
        ToolStripActionList.cc
        ToolStripActionList.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

set_source_files_properties(OverlayBackdrop.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

qt_add_library(QGroundControlControlsModule STATIC)

qt_add_qml_module(QGroundControlControlsModule
    URI QGroundControl.Controls
    VERSION 1.0
    RESOURCE_PREFIX /qml
    QML_FILES
        AltModeMenu.qml
        AppSettings.qml
        APMSubMotorDisplay.qml
        AppMessages.qml
        ArrangeableOverlayItem.qml
        AutotuneUI.qml
        AxisMonitor.qml
        CameraSwitchButton.qml
        ClickableColor.qml
        DeadMouseArea.qml
        DragToPosition.qml
        DropButton.qml
        DropPanel.qml
        EditPositionDialog.qml
        ExclusiveGroupItem.qml
        FactSlider.qml
        FactSliderPanel.qml
        FileButton.qml
        FlightModeMenu.qml
        HeightIndicator.qml
        HorizontalFactValueGrid.qml
        SetupProgressBar.qml
        SetupSheet.qml
        InstrumentValueEditDialog.qml
        InstrumentValueLabel.qml
        InstrumentValueValue.qml
        JiggleAnimation.qml
        JoystickThumbPad.qml
        KMLOrSHPFileDialog.qml
        LabelledButton.qml
        LabelledComboBox.qml
        LabelledFactMenu.qml
        LabelledLabel.qml
        LabelledSlider.qml
        LogReplayStatusBar.qml
        MainWindowSavedState.qml
        MAVLinkChart.qml
        MAVLinkMessageButton.qml
        MissionCommandDialog.qml
        MissionItemIndexLabel.qml
        MvPanelPage.qml
        OfflineMapButton.qml
        OverlayActivityCapsule.qml
        OverlayActivityRing.qml
        OverlayEditBadge.qml
        OverlayEditSlot.qml
        OverlayDisclosureRow.qml
        OverlayMenuItem.qml
        OverlayMenuSeparator.qml
        OverlayBackdrop.qml
        OverlayCaption.qml
        OverlayCapsule.qml
        OverlayGlass.qml
        OverlayPill.qml
        OverlayPopover.qml
        OverlayRoundButton.qml
        OverlaySegmentedControl.qml
        OverlayRig.qml
        OverlayViewSwitch.qml
        OverlayShadowEffect.qml
        ParameterDiffDialog.qml
        ParameterEditor.qml
        ParameterEditorDialog.qml
        PIDTuning.qml
        PipState.qml
        PipView.qml
        PreFlightCheckButton.qml
        PreFlightCheckGroup.qml
        PreFlightCheckModel.qml
        QGCButton.qml
        QGCCheckBox.qml
        QGCCheckBoxSlider.qml
        QGCColoredImage.qml
        QGCColumnButton.qml
        QGCComboBox.qml
        QGCDynamicObjectManager.qml
        QGCFileDialog.qml
        QGCFlickable.qml
        QGCFlickableScrollIndicator.qml
        QGCGroupBox.qml
        QGCLabel.qml
        QGCSpinner.qml
        QGCListView.qml
        QGCMapLabel.qml
        QGCMarqueeLabel.qml
        QGCMenu.qml
        QGCMenuItem.qml
        QGCMenuSeparator.qml
        QGCMouseArea.qml
        QGCMovableItem.qml
        QGCOptionsComboBox.qml
        QGCPageIndicator.qml
        QGCPopupDialog.qml
        QGCRadioButton.qml
        QGCRoundButton.qml
        QGCSimpleMessageDialog.qml
        QGCSlider.qml
        QGCSwipeView.qml
        QGCSwitch.qml
        QGCTabBar.qml
        QGCTabButton.qml
        QGCTextField.qml
        QGCToolBarButton.qml
        QGCToolTip.qml
        QGCToolInsets.qml
        RCChannelMonitor.qml
        RCToParamDialog.qml
        SectionHeader.qml
        SelectableControl.qml
        SettingsButton.qml
        SettingsGroupLayout.qml
        SetupPage.qml
        SliderSwitch.qml
        SubMenuButton.qml
        TimedProgressTracker.qml
        ToolIndicatorPage.qml
        ToolStrip.qml
        ToolStripDropPanel.qml
        ToolStripHoverButton.qml
        ValueSlider.qml
        VehicleRotationCal.qml
        VehicleSummaryRow.qml

# Due to the fact t hatset_source_files_properties with QT_RESOURCE_ALIAS doesn't work in cmombination with qt_qdd_add_qml_module on Windows.
# We had to move various qml files to this directory to get things to work. We will revisit this in the 5.1 release.

        # src/AnalyzeView/
        AnalyzePage.qml

        # src/FirstRunPromptDialogs/
        FirstRunPrompt.qml

        # src/PlanView/
        CameraCalcCamera.qml
        CameraCalcGrid.qml
        CameraSection.qml
        CorridorScanEditor.qml
        CorridorScanMapVisual.qml
        FWLandingPatternEditor.qml
        FWLandingPatternMapVisual.qml
        GeoFenceEditor.qml
        GeoFenceMapVisuals.qml
        MissionItemEditor.qml
        MissionItemMapVisual.qml
        MissionItemStatus.qml
        MissionSettingsEditor.qml
        PlanEditToolbar.qml
        PlanEditMapItems.qml
        PlanGroupCard.qml
        PlanGroupRow.qml
        ToolDrawerPage.qml
        PlanFactRow.qml
        PlanSectionLabel.qml
        PlanSwitchRow.qml
        PlanToolBarIndicators.qml
        PlanView.qml
        RallyPointEditor.qml
        RallyPointMapVisuals.qml
        SimpleItemEditor.qml
        SimpleItemMapVisual.qml
        StructureScanEditor.qml
        StructureScanMapVisual.qml
        SurveyItemEditor.qml
        SurveyMapVisual.qml
        TakeoffItemMapVisual.qml
        TerrainStatus.qml
        TransectStyleComplexItemEditor.qml
        TransectStyleComplexItemStats.qml
        TransectStyleComplexItemTabBar.qml
        TransectStyleComplexItemTerrainFollow.qml
        TransectStyleMapVisuals.qml
        VTOLLandingPatternEditor.qml
        VTOLLandingPatternMapVisual.qml

        # src/UI/preferences/
        OfflineMapInfo.qml

        # src/UI/toolbar/
        BatteryGlyph.qml
        GcsBatteryGlyph.qml
        BatteryIndicator.qml
        FlightModeIndicator.qml
        FlightModeMenuIndicator.qml
        FlyViewToolBar.qml
        FlyViewToolBarIndicators.qml
        GPSIndicator.qml
        GPSIndicatorPage.qml
        MainStatusIndicator.qml
        VehicleStatusSummary.qml
        MainStatusIndicatorOfflinePage.qml
        PlanViewToolBar.qml
        RemoteIDIndicatorPage.qml
        SignalStrength.qml
        VehicleMessageList.qml

        # src/QtLocationPlugin/QMLControl/
        OfflineMapEditor.qml

    NO_PLUGIN # Required by Qt 6.6.3. TODO: Remove when no longer supported.
)

qt_add_shaders(QGroundControlControlsModule "qgc_glass_shaders"
    PREFIX "/shaders"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}/Shaders"
    FILES Shaders/LiquidGlass.frag
)

qt_add_library(ScreenToolsModule STATIC)

set_source_files_properties(ScreenTools.qml PROPERTIES
    QT_QML_SINGLETON_TYPE TRUE
)

qt_add_qml_module(ScreenToolsModule
    URI QGroundControl.ScreenTools
    VERSION 1.0
    RESOURCE_PREFIX /qml
    QML_FILES
        ScreenTools.qml
    NO_PLUGIN # Required by Qt 6.6.3. TODO: Remove when no longer supported.
)

