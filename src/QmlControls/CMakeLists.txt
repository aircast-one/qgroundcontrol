# ============================================================================
# QML Controls Module
# Reusable QML UI controls and controllers
# ============================================================================

target_sources(${CMAKE_PROJECT_NAME}
    PRIVATE
        ColoredSvgImageProvider.cc
        ColoredSvgImageProvider.h
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
        ObjectItemModelBase.cc
        ObjectItemModelBase.h
        ObjectListModelBase.cc
        ObjectListModelBase.h
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
        QmlObjectTreeModel.cc
        QmlObjectTreeModel.h
        QmlUnitsConversion.h
        RCChannelMonitorController.cc
        RCChannelMonitorController.h
        RCToParamDialogController.cc
        RCToParamDialogController.h
        ScreenToolsController.cc
        ScreenToolsController.h
        ServoOutputMonitorController.cc
        ServoOutputMonitorController.h
        TerrainProfile.cc
        TerrainProfile.h
        ToolStripAction.cc
        ToolStripAction.h
        ToolStripActionList.cc
        ToolStripActionList.h
        TransformPositionController.cc
        TransformPositionController.h
)

target_include_directories(${CMAKE_PROJECT_NAME} PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})

# ----------------------------------------------------------------------------
# QML Controls Module
# ----------------------------------------------------------------------------
qt_add_library(QGroundControlControlsModule STATIC)

set_source_files_properties(OverlayBackdrop.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)
set_source_files_properties(ScreenTools.qml PROPERTIES QT_QML_SINGLETON_TYPE TRUE)

set(_qgc_controls_qml
    AltFrameCombo.qml
    AltFrameDialog.qml
    AltModeMenu.qml
    APMSubMotorDisplay.qml
    AppSettings.qml
    ArrangeableOverlayItem.qml
    AutotuneUI.qml
    AxisMonitor.qml
    BatteryGlyph.qml
    CameraSwitchButton.qml
    ClickableColor.qml
    ConfigSection.qml
    ConnectionStatusRow.qml
    DeadMouseArea.qml
    DragToPosition.qml
    DropButton.qml
    DropPanel.qml
    EditPositionDialog.qml
    ExclusiveGroupItem.qml
    FactSlider.qml
    FileButton.qml
    FixStatusDot.qml
    FlightModeMenu.qml
    GcsBatteryGlyph.qml
    HeightIndicator.qml
    HorizontalFactValueGrid.qml
    IndicatorButton.qml
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
    MissionCommandDialog.qml
    MvPanelPage.qml
    OfflineMapButton.qml
    OverlayActivityCapsule.qml
    OverlayActivityRing.qml
    OverlayBackdrop.qml
    OverlayCapsule.qml
    OverlayCaption.qml
    OverlayDisclosureRow.qml
    OverlayEditBadge.qml
    OverlayEditSlot.qml
    OverlayGlass.qml
    OverlayMenuItem.qml
    OverlayMenuSeparator.qml
    OverlayPill.qml
    OverlayPopover.qml
    OverlayRig.qml
    OverlayRoundButton.qml
    OverlaySegmentedControl.qml
    OverlayShadowEffect.qml
    OverlayViewSwitch.qml
    ParameterDiffDialog.qml
    ParameterEditor.qml
    ParameterEditorDialog.qml
    PIDTuning.qml
    PipState.qml
    PipView.qml
    PlanEditMapItems.qml
    PlanFactRow.qml
    PlanGroupCard.qml
    PlanGroupRow.qml
    PlanSectionLabel.qml
    PlanSwitchRow.qml
    PreFlightCheckButton.qml
    PreFlightCheckGroup.qml
    PreFlightCheckModel.qml
    QGCButton.qml
    QGCCheckBox.qml
    QGCCheckBoxSlider.qml
    QGCColoredImage.qml
    QGCColumnButton.qml
    QGCComboBox.qml
    QGCDelayButton.qml
    QGCDynamicObjectManager.qml
    QGCFileDialog.qml
    QGCFlickable.qml
    QGCFlickableScrollIndicator.qml
    QGCGroupBox.qml
    QGCLabel.qml
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
    QGCPopupDialogFactory.qml
    QGCRadioButton.qml
    QGCRoundButton.qml
    QGCSimpleMessageDialog.qml
    QGCSlider.qml
    QGCSpinner.qml
    QGCSwipeView.qml
    QGCSwitch.qml
    QGCTabBar.qml
    QGCTabButton.qml
    QGCTextField.qml
    QGCToolBarButton.qml
    QGCToolInsets.qml
    QGCToolTip.qml
    QGCVectorImage.qml
    RallyPointEditor.qml
    RCChannelMonitor.qml
    RCToParamDialog.qml
    ResizeHandle.qml
    ScreenTools.qml
    SectionHeader.qml
    SelectableControl.qml
    SettingsButton.qml
    SettingsGroupLayout.qml
    SetupPage.qml
    SetupProgressBar.qml
    SetupSheet.qml
    SliderSwitch.qml
    SubMenuButton.qml
    TimedProgressTracker.qml
    ToolDrawerPage.qml
    ToolIndicatorPage.qml
    ToolStrip.qml
    ToolStripDropPanel.qml
    ToolStripHoverButton.qml
    ValueSlider.qml
    VehicleRotationCal.qml
    VehicleStatusSummary.qml
    VehicleSummaryRow.qml
    VideoSourceMenu.qml
)

qt_add_qml_module(QGroundControlControlsModule
    URI QGroundControl.Controls
    VERSION 1.0
    RESOURCE_PREFIX /qml
    QML_FILES
        ${_qgc_controls_qml}
    NO_PLUGIN
)

qt_add_shaders(QGroundControlControlsModule "qgc_glass_shaders"
    PREFIX "/shaders"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}/Shaders"
    FILES Shaders/LiquidGlass.frag
)

qgc_add_json_resources(json_controls)

qt_add_resources(QGroundControlControlsModule qmlcontrols_resources
    PREFIX "/res"
    BASE "${CMAKE_SOURCE_DIR}/resources"
    FILES
        "${CMAKE_SOURCE_DIR}/resources/ArrowRight.svg"
        "${CMAKE_SOURCE_DIR}/resources/buttonRight.svg"
        "${CMAKE_SOURCE_DIR}/resources/chevron-down.svg"
        "${CMAKE_SOURCE_DIR}/resources/chevron-up.svg"
        "${CMAKE_SOURCE_DIR}/resources/clockwise-arrow.svg"
        "${CMAKE_SOURCE_DIR}/resources/counter-clockwise-arrow.svg"
        "${CMAKE_SOURCE_DIR}/resources/JoystickBezelLight.png"
)

# subMenuButtonImage.png aliases the shared CogWheels.png at the repo root.
set_source_files_properties("${CMAKE_SOURCE_DIR}/resources/CogWheels.png"
    PROPERTIES QT_RESOURCE_ALIAS "subMenuButtonImage.png")

qt_add_resources(QGroundControlControlsModule qmlcontrols_qmlimages
    PREFIX "/qmlimages"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}"
    FILES
        "${CMAKE_CURRENT_SOURCE_DIR}/arrow-down.png"
        "${CMAKE_CURRENT_SOURCE_DIR}/checkbox-check.svg"
        "${CMAKE_SOURCE_DIR}/resources/CogWheels.png"
)

# Globbed because FactValueGrid enumerates :/InstrumentValueIcons/ at runtime
# to build the icon picker — every SVG in the directory must be embedded.
file(GLOB INSTRUMENT_VALUE_ICONS CONFIGURE_DEPENDS
    "${CMAKE_SOURCE_DIR}/resources/InstrumentValueIcons/*.svg"
)
qt_add_resources(QGroundControlControlsModule instrument_value_icons
    PREFIX "/InstrumentValueIcons"
    BASE "${CMAKE_SOURCE_DIR}/resources/InstrumentValueIcons"
    FILES ${INSTRUMENT_VALUE_ICONS}
)
