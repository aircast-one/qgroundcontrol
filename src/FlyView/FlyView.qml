import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

import QtLocation
import QtPositioning
import QtQuick.Window
import QtQml.Models

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlyView
import QGroundControl.FlightMap
import QGroundControl.Toolbar
import QGroundControl.Viewer3D

Item {
    id: _root

    readonly property bool _is3DMode:       QGCViewer3DManager.displayMode === QGCViewer3DManager.View3D
    readonly property bool _keepSceneAlive: QGroundControl.settingsManager.viewer3DSettings.keepSceneAlive.rawValue

    // These should only be used by MainRootWindow
    function switchVideoSource(anchorItem, x, y) {
        // Two cameras: one tap cycles between them. Three or more: open the menu so the
        // target is one tap away rather than several cycles.
        if (QGroundControl.videoManager.videoSourceCount > 2) {
            videoSourceMenu.parent = anchorItem
            videoSourceMenu.open(x, y)
        } else {
            QGroundControl.videoManager.switchActiveVideoSource()
        }
    }

    property var planController:    _planController
    property var guidedController:  _guidedController

    PlanMasterController {
        id:                     _planController
        flyView:                true
        Component.onCompleted:  start()
    }

    property bool   _mainWindowIsMap:       mapControl.pipState.state === mapControl.pipState.fullState
    property bool   _isFullWindowItemDark:  _mainWindowIsMap ? mapControl.isSatelliteMap : true
    property var    _activeVehicle:         QGroundControl.multiVehicleManager.activeVehicle
    property var    _missionController:     _planController.missionController
    property var    _geoFenceController:    _planController.geoFenceController
    property var    _rallyPointController:  _planController.rallyPointController
    property real   _margins:               ScreenTools.defaultFontPixelWidth / 2
    property var    _guidedController:      guidedActionsController
    property var    _guidedValueSlider:     guidedValueSlider
    property var    _widgetLayer:           widgetLayer
    property real   _toolsMargin:           ScreenTools.defaultFontPixelWidth * 0.75
    property rect   _centerViewport:        Qt.rect(0, 0, width, height)
    property real   _rightPanelWidth:       ScreenTools.defaultFontPixelWidth * 30
    property var    _mapControl:            mapControl
    property real   _widgetMargin:          ScreenTools.defaultFontPixelWidth * 0.75

    property real   _fullItemZorder:    0
    property real   _pipItemZorder:     QGroundControl.zOrderWidgets

    function _calcCenterViewPort() {
        var newToolInset = Qt.rect(0, 0, width, height)
        toolstrip.adjustToolInset(newToolInset)
    }

    function dropMainStatusIndicatorTool() {
        toolbar.dropMainStatusIndicatorTool();
    }

    QGCToolInsets {
        id:                     _toolInsets
        topEdgeLeftInset:       toolbar.height
        topEdgeCenterInset:     topEdgeLeftInset
        topEdgeRightInset:      topEdgeLeftInset
        leftEdgeBottomInset:    _pipView.leftEdgeBottomInset
        bottomEdgeLeftInset:    _pipView.bottomEdgeLeftInset
    }

    Item {
        id:                 mapHolder
        anchors.fill:       parent

        FlyViewMap {
            id:                     mapControl
            planMasterController:   _planController
            rightPanelWidth:        ScreenTools.defaultFontPixelHeight * 9
            pipView:                _pipView
            pipMode:                !_mainWindowIsMap
            toolInsets:             customOverlay.totalToolInsets
            mapName:                "FlightDisplayView"
            enabled:                !_is3DMode
            visible:                !_is3DMode
        }

        FlyViewVideo {
            id:         videoControl
            pipView:    _pipView
        }

        PipView {
            id:                     _pipView
            objectName:             "pipView"
            margin:                 _toolsMargin
            item1IsFullSettingsKey: "MainFlyWindowIsMap"
            item1:                  mapControl
            item2:                  QGroundControl.videoManager.hasVideo ? videoControl : null
            show:                   QGroundControl.videoManager.hasVideo && !QGroundControl.videoManager.fullScreen &&
                                        (videoControl.pipState.state === videoControl.pipState.pipState || mapControl.pipState.state === mapControl.pipState.pipState)
            z:                      QGroundControl.zOrderWidgets

            // Show a camera-switch button on the pip only while the video is the pip item
            showActionButton:       QGroundControl.videoManager.hasMultipleVideoSources &&
                                        videoControl.pipState.state === videoControl.pipState.pipState
            actionButtonText:       QGroundControl.videoManager.activeSourceLabel
            onActionButtonClicked:  _root.switchVideoSource(videoControl, videoControl.width - videoSourceMenu.width, videoControl.height)

            property real leftEdgeBottomInset: visible && !hasCustomPosition ? width + _toolsMargin : 0
            property real bottomEdgeLeftInset: visible && !hasCustomPosition ? height + _toolsMargin : 0
        }

        FlyViewWidgetLayer {
            id:                     widgetLayer
            anchors.top:            parent.top
            anchors.bottom:         parent.bottom
            anchors.left:           parent.left
            anchors.right:          guidedValueSlider.visible ? guidedValueSlider.left : parent.right
            anchors.margins:        _widgetMargin
            anchors.topMargin:      toolbar.height + _widgetMargin
            z:                      _fullItemZorder + 2
            parentToolInsets:       _toolInsets
            mapControl:             _mapControl
            viewer3DCameraController: viewer3DLoader.item ? viewer3DLoader.item.cameraController : null
            visible:                !QGroundControl.videoManager.fullScreen
        }

        FlyViewCustomLayer {
            id:                 customOverlay
            anchors.fill:       widgetLayer
            z:                  _fullItemZorder + 2
            parentToolInsets:   widgetLayer.totalToolInsets
            mapControl:         _mapControl
            visible:            !QGroundControl.videoManager.fullScreen
        }

        // Camera switch button for the full-screen video. Placed here (above the instrument
        // overlays) so it is not hidden behind them. The small-pip case is handled by PipView.
        CameraSwitchButton {
            id:                         fullVideoCameraSwitchButton
            z:                          _fullItemZorder + 3
            anchors.top:                parent.top
            anchors.horizontalCenter:   parent.horizontalCenter
            anchors.topMargin:          ScreenTools.defaultFontPixelHeight
            opacity:                    0.75
            visible:                    QGroundControl.videoManager.hasMultipleVideoSources &&
                                        videoControl.pipState.state === videoControl.pipState.fullState
            text:                       QGroundControl.videoManager.activeSourceLabel
            onClicked:                  _root.switchVideoSource(fullVideoCameraSwitchButton, 0, fullVideoCameraSwitchButton.height)
        }

        VideoSourceMenu {
            id: videoSourceMenu
        }

        // Development tool for visualizing the insets for a paticular layer, show if needed
        FlyViewInsetViewer {
            id:                     widgetLayerInsetViewer
            anchors.top:            parent.top
            anchors.bottom:         parent.bottom
            anchors.left:           parent.left
            anchors.right:          guidedValueSlider.visible ? guidedValueSlider.left : parent.right
            z:                      widgetLayer.z + 1
            insetsToView:           widgetLayer.totalToolInsets
            visible:                false
        }

        GuidedActionsController {
            id:                 guidedActionsController
            missionController:  _missionController
            guidedValueSlider:     _guidedValueSlider
        }

        //-- Guided value slider (e.g. altitude)
        GuidedValueSlider {
            id:                 guidedValueSlider
            anchors.right:      parent.right
            anchors.top:        parent.top
            anchors.bottom:     parent.bottom
            anchors.topMargin:  toolbar.height
            z:                  QGroundControl.zOrderTopMost
            visible:            false
        }

        Loader {
            id:           viewer3DLoader
            z:            1
            anchors.fill: parent
            visible:      _is3DMode
        }

        Connections {
            target: QGCViewer3DManager
            function onDisplayModeChanged() {
                if (QGCViewer3DManager.displayMode === QGCViewer3DManager.View3D) {
                    if (!viewer3DLoader.item) {
                        viewer3DLoader.setSource("qrc:/qml/QGroundControl/Viewer3D/Models3D/Viewer3DModel.qml")
                    }
                } else if (!_keepSceneAlive) {
                    viewer3DLoader.source = ""
                }
            }
        }
    }

    FlyViewToolBar {
        id:                 toolbar
        guidedValueSlider:  _guidedValueSlider
        visible:            !QGroundControl.videoManager.fullScreen
        dockedTelemetryBar: telemetryValuesBar.dockedInToolbar ? telemetryValuesBar : null
    }

    TelemetryValuesBar {
        id:                     telemetryValuesBar
        objectName:             "telemetryValuesBar"
        settingsGroup:          factValueGrid.telemetryBarSettingsGroup
        specificVehicleForCard: null
        visible:                toolbar.visible

        readonly property bool dockedInToolbar: y < ScreenTools.toolbarHeight

        DragToPosition {
            id:                 telemetryBarDragPosition
            target:             telemetryValuesBar
            settingsKeyPrefix:  "TelemetryValuesBar"
            defaultX:           (_root.width - telemetryValuesBar.width) / 2
            defaultY:           0
        }

        DragHandler {
            onActiveChanged: {
                if (!active) {
                    if (telemetryValuesBar.dockedInToolbar) {
                        telemetryValuesBar.y = 0
                        telemetryValuesBar.x = Math.max(toolbar.dockAreaLeft,
                                                        Math.min(telemetryValuesBar.x, toolbar.dockAreaRight - telemetryValuesBar.width))
                    }
                    telemetryBarDragPosition.commit()
                }
            }
        }
    }
}
