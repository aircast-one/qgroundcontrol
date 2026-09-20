# ============================================================================
# Flight Display Module
# In-flight view, telemetry displays, and vehicle control UI
# ============================================================================

qgc_qml_only_library(FlyViewModule
    URI QGroundControl.FlyView
    VERSION 1.0
    RESOURCE_PREFIX /qml
    QML_FILES
        CustomGuidedActionsController.qml
        CameraAimArea.qml
        CameraControlLayer.qml
        CameraDockPill.qml
        CameraEdgeSlider.qml

        CameraShutterButton.qml
        DefaultChecklist.qml
        DetectionOverlayVideo.qml
        FixedWingChecklist.qml
        FlightDisplayViewDummy.qml
        FlightDisplayViewVideoOutput.qml
        FlightDisplayViewQtMultimedia.qml
        FlightDisplayViewUVC.qml
        FlightDisplayViewVideo.qml
        FlyView.qml
        FlyViewAdditionalActionsButton.qml
        FlyViewAdditionalActionsList.qml
        FlyViewAdditionalActionsPanel.qml
        FlyViewAdditionalCustomActionsList.qml
        FlyViewCustomLayer.qml
        FlyViewInsetViewer.qml
        FlyViewInstrumentPanel.qml
        FlyViewGripperButton.qml
        FlyViewGripperDropPanel.qml
        FlyViewMap.qml
        FlyViewMissionCompleteDialog.qml
        FlyViewPreFlightChecklistPopup.qml
        FlyViewToolStrip.qml
        FlyViewToolStripActionList.qml

        FlyViewTopRightColumnLayout.qml
        FlyViewTopRightPanel.qml
        FlyViewVideo.qml
        FlyViewWidgetLayer.qml
        GuidedActionConfirm.qml
        GuidedActionGripper.qml
        GuidedActionLand.qml
        GripperMenu.qml
        GuidedActionPause.qml
        GuidedActionRTL.qml
        GuidedActionsController.qml
        GuidedToolStripAction.qml
        GuidedValueSlider.qml
        MultiRotorChecklist.qml
        MultiVehicleList.qml
        ObstacleDistanceOverlay.qml
        ObstacleDistanceOverlayMap.qml
        ObstacleDistanceOverlayVideo.qml
        OnScreenCameraTrackingController.qml
        OnScreenGimbalController.qml
        PreFlightBatteryCheck.qml
        PreFlightCheckList.qml
        PreFlightCheckListShowAction.qml
        PreFlightGPSCheck.qml
        PreFlightRCCheck.qml
        PreFlightSensorsHealthCheck.qml
        PreFlightSoundCheck.qml
        ProximityRadarValues.qml
        ProximityRadarVideoView.qml
        RcControlsLayer.qml
        RoverChecklist.qml
        SubChecklist.qml
        TelemetryChipsLayer.qml
        TelemetryValuesBar.qml
        TerrainProgress.qml
        VehicleWarnings.qml
        VideoTilesLayer.qml
        VirtualJoystick.qml
        VTOLChecklist.qml
    NO_PLUGIN
)

qgc_qml_resources(FlyViewModule flyview_resources
    PREFIX "/res"
    BASE "${CMAKE_SOURCE_DIR}/resources"
    FILES
        "${CMAKE_SOURCE_DIR}/resources/Gripper.svg"
        "${CMAKE_SOURCE_DIR}/resources/GripperGrab.svg"
        "${CMAKE_SOURCE_DIR}/resources/GripperRelease.svg"
        "${CMAKE_SOURCE_DIR}/resources/Gripper.svg"
        "${CMAKE_SOURCE_DIR}/resources/land.svg"
        "${CMAKE_SOURCE_DIR}/resources/NoVideoBackground.jpg"
        "${CMAKE_SOURCE_DIR}/resources/pause-mission.svg"
)

qgc_qml_resources(FlyViewModule flyview_qmlimages
    PREFIX "/qmlimages"
    BASE "${CMAKE_SOURCE_DIR}/resources"
    FILES
        "${CMAKE_SOURCE_DIR}/resources/check.svg"
)

qgc_qml_resources(FlyViewModule flyview_images
    PREFIX "/qmlimages"
    BASE "${CMAKE_CURRENT_SOURCE_DIR}/Images"
    FILES
        Images/CameraLight.svg
        Images/CameraRecenter.svg
        Images/CameraTilt.svg
        Images/CameraYawFollow.svg
        Images/CameraYawLock.svg
        Images/CameraZoom.svg
        Images/RcOverride.svg
        Images/StatusRC.svg
        Images/StatusSat.svg
)
