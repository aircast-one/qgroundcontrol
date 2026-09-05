/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Effects
import QtQuick.Layouts

import QtLocation
import QtPositioning
import QtQuick.Window
import QtQml.Models

import QGroundControl
import QGroundControl.Controllers
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap
import QGroundControl.Palette
import QGroundControl.ScreenTools
import QGroundControl.Vehicle

// 3D Viewer modules
import Viewer3D

Item {
    id: _root

    // These should only be used by MainRootWindow
    property var planController:    _planController
    property var guidedController:  _guidedController
    property var overlayRig:        _overlayRig

    OverlayRig {
        id:         _overlayRig
        objectName: "overlayRigFlyView"
        viewport:   _root
        topInset:   toolbar.height
    }

    Shortcut {
        sequence:       "Escape"
        enabled:        _overlayRig.editMode
        onActivated:    _overlayRig.editMode = false
    }

    // Properties of UTM adapter
    property bool utmspSendActTrigger: false

    PlanMasterController {
        id:                     _planController
        flyView:                true
        Component.onCompleted:  start()
    }

    property bool   _mainWindowIsMap:       mapControl.pipState.state === mapControl.pipState.fullState
    property bool   _isFullWindowItemDark:  _mainWindowIsMap ? mapControl.isSatelliteMap : true

    // The glass keys off the same signal the map chrome already trusts: a street map is a light
    // backdrop, satellite and video are dark. Wrong material is worse than no material, so
    // anything we cannot classify stays dark.
    Binding {
        target:     OverlayBackdrop
        property:   "isDark"
        value:      _root._isFullWindowItemDark
    }

    // Everything that can move inside the captured layers: the video, the instruments a
    // connected vehicle drives (attitude, battery, telemetry all sit in mapHolder), and the
    // jiggle while arranging. With none of them running the backdrop cannot change.
    Binding {
        target:     OverlayBackdrop
        property:   "sourceAnimating"
        value:      QGroundControl.videoManager.decoding ||
                        QGroundControl.multiVehicleManager.activeVehicle !== null ||
                        _overlayRig.editMode
    }

    // A map pan or zoom changes the backdrop without any video running, so it asks for a single
    // refresh rather than keeping the pulse alive.
    Connections {
        target:                 mapControl
        ignoreUnknownSignals:   true
        function onCenterChanged()    { OverlayBackdrop.refresh() }
        function onZoomLevelChanged() { OverlayBackdrop.refresh() }
    }
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
        leftEdgeBottomInset:    _pipView.leftEdgeBottomInset
        bottomEdgeLeftInset:    _pipView.bottomEdgeLeftInset
        // The status floats over the picture now, so the map's own widgets have to keep clear
        // of it themselves -- nothing pushes them down any more.
        topEdgeLeftInset:       toolbar.height
        topEdgeCenterInset:     toolbar.height
        topEdgeRightInset:      toolbar.height
    }

    FlyViewToolBar {
        id:                 toolbar
        z:                  QGroundControl.zOrderWidgets
        visible:            !QGroundControl.videoManager.fullScreen
    }

    Item {
        id:                 mapHolder
        anchors.top:        parent.top
        anchors.bottom:     parent.bottom
        anchors.left:       parent.left
        anchors.right:      parent.right

        Item {
            id:             backdropContent
            objectName:     "backdropContent"
            anchors.fill:   parent

            FlyViewMap {
                id:                     mapControl
                planMasterController:   _planController
                rightPanelWidth:        ScreenTools.defaultFontPixelHeight * 9
                pipView:                _pipView
                pipMode:                !_mainWindowIsMap
                toolInsets:             customOverlay.totalToolInsets
                mapName:                "FlightDisplayView"
                enabled:                !viewer3DWindow.isOpen
            }

            FlyViewVideo {
                id:         videoControl
                overlayRig: _overlayRig
                pipView:    _pipView
            }
        }

        PipView {
            id:                     _pipView
            objectName:             "pipView"
            fullContentItem:        backdropContent
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
            actionButtonText:       QGroundControl.videoManager.cameraName(QGroundControl.videoManager.activeVideoSource)
            onActionButtonClicked:  QGroundControl.videoManager.switchActiveVideoSource()

            widthOverride:          videoTilesLayer.pipWidthOverride

            property real leftEdgeBottomInset: visible && !hasCustomPosition ? width + videoTilesLayer.dockExtent + _toolsMargin : 0
            property real bottomEdgeLeftInset: visible && !hasCustomPosition ? height + _toolsMargin : 0

            overlayRig:            _overlayRig
            Component.onCompleted:   _overlayRig.registerAnchor(_pipView, _pipView.dragToPosition)
            Component.onDestruction: _overlayRig.unregisterMovable(_pipView)
        }

        FlyViewWidgetLayer {
            id:                     widgetLayer
            overlayRig:             _overlayRig
            anchors.top:            parent.top
            anchors.bottom:         parent.bottom
            anchors.left:           parent.left
            anchors.right:          guidedValueSlider.visible ? guidedValueSlider.left : parent.right
            z:                      _fullItemZorder + 2 // we need to add one extra layer for map 3d viewer (normally was 1)
            parentToolInsets:       _toolInsets
            mapControl:             _mapControl
            visible:                !QGroundControl.videoManager.fullScreen
            utmspActTrigger:        utmspSendActTrigger
            isViewer3DOpen:         viewer3DWindow.isOpen
        }

        FlyViewCustomLayer {
            id:                 customOverlay
            anchors.fill:       widgetLayer
            z:                  _fullItemZorder + 2
            parentToolInsets:   widgetLayer.totalToolInsets
            mapControl:         _mapControl
            visible:            !QGroundControl.videoManager.fullScreen
        }

        CameraControlLayer {
            id:                 cameraControlLayer
            objectName:         "cameraControlLayer"
            overlayRig:         _overlayRig
            anchors.fill:       parent
            videoIsMainItem:    !_mainWindowIsMap
            z:                  _fullItemZorder + 1
        }

        VideoTilesLayer {
            id:             videoTilesLayer
            objectName:     "videoTilesLayer"
            overlayRig:     _overlayRig
            pipView:        _pipView
            topInset:       toolbar.height
            anchors.fill:   parent
            z:              _fullItemZorder + 3
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
            text:                       QGroundControl.videoManager.cameraName(QGroundControl.videoManager.activeVideoSource)
            onClicked:                  QGroundControl.videoManager.switchActiveVideoSource()
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
            z:                  QGroundControl.zOrderTopMost
            visible:            false
        }

        Viewer3D{
            id:                     viewer3DWindow
            anchors.fill:           parent
        }

        MouseArea {
            id:             editModeExitCatcher
            anchors.fill:   parent
            z:              _fullItemZorder + 0.5
            visible:        _overlayRig.editMode
            onClicked: (mouse) => {
                const p = editModeExitCatcher.mapToItem(_overlayRig.viewport, mouse.x, mouse.y)
                if (!_overlayRig.hitTest(p.x, p.y)) {
                    _overlayRig.editMode = false
                }
            }
        }
    }

    ShaderEffectSource {
        id:           backdropCapture
        width:        backdropContent.width
        height:       backdropContent.height
        sourceItem:   backdropContent
        live:         false
        visible:      false
        textureSize:  Qt.size(Math.max(1, Math.round(backdropContent.width  / _backdropDownscale)),
                              Math.max(1, Math.round(backdropContent.height / _backdropDownscale)))

        readonly property int _backdropDownscale: 4
    }

    MultiEffect {
        id:            frostedBackdrop
        width:         backdropContent.width
        height:        backdropContent.height
        source:        backdropCapture
        blurEnabled:   true
        blur:          1.0
        blurMax:       32
        saturation:    0.25
        visible:       false
        layer.enabled: true

        Component.onCompleted: {
            OverlayBackdrop.contentSource   = backdropContent
            OverlayBackdrop.contentBackdrop = frostedBackdrop
        }

        Connections {
            target: OverlayBackdrop
            function onRefreshed() {
                backdropCapture.scheduleUpdate()
                chromeCapture.scheduleUpdate()
            }
        }
    }

    ShaderEffectSource {
        id:           chromeCapture
        width:        mapHolder.width
        height:       mapHolder.height
        sourceItem:   mapHolder
        live:         false
        visible:      false
        textureSize:  Qt.size(Math.max(1, Math.round(mapHolder.width  / 4)),
                              Math.max(1, Math.round(mapHolder.height / 4)))
    }

    MultiEffect {
        id:            frostedBackdropFull
        width:         mapHolder.width
        height:        mapHolder.height
        source:        chromeCapture
        blurEnabled:   true
        blur:          1.0
        blurMax:       32
        saturation:    0.25
        visible:       false
        layer.enabled: true

        Component.onCompleted: {
            OverlayBackdrop.fullSource   = mapHolder
            OverlayBackdrop.fullBackdrop = frostedBackdropFull
        }
    }

    TelemetryChipsLayer {
        id:             telemetryChipsLayer
        objectName:     "telemetryChipsLayer"
        overlayRig:     _overlayRig
        anchors.fill:   parent
        visible:        toolbar.visible
        z:              QGroundControl.zOrderWidgets
    }

    // The one place that says what this mode is, how it works, and how to leave it. Edit mode
    // used to announce itself only by making things wobble, and carried two separate controls
    // both labelled "Done".
    Rectangle {
        id:                         editModeDonePill
        objectName:                 "editModeDonePill"
        anchors.top:                parent.top
        anchors.topMargin:          toolbar.height + ScreenTools.defaultFontPixelHeight
        anchors.horizontalCenter:   parent.horizontalCenter
        z:                          QGroundControl.zOrderTopMost
        visible:                    _overlayRig.editMode
        width:                      editModeRow.width + ScreenTools.defaultFontPixelWidth * 4
        height:                     ScreenTools.defaultFontPixelHeight * 2.4
        radius:                     height / 2
        color:                      "transparent"

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.radius
        }

        Row {
            id:                 editModeRow
            anchors.centerIn:   parent
            spacing:            ScreenTools.defaultFontPixelWidth * 2

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                color:                  QGroundControl.globalPalette.colorGrey
                font.pointSize:         ScreenTools.smallFontPointSize
                // Each window size keeps its own arrangement, and nothing else on screen says
                // so - without this the layout looks lost every time the window is resized.
                text: qsTr("Drag to arrange · Tap the eye to hide · Layout for %1×%2")
                          .arg(Math.round(_root.width)).arg(Math.round(_root.height))
            }

            OverlayMenuItem {
                anchors.verticalCenter: parent.verticalCenter
                objectName:             "editModeAddRcControlButton"
                text:                   qsTr("Add RC control…")
                onClicked: {
                    _overlayRig.editMode = false
                    mainWindow.showSettingsTool(qsTr("Fly View"))
                }
            }

            OverlayMenuItem {
                anchors.verticalCenter: parent.verticalCenter
                objectName:             "editModeDoneButton"
                text:                   qsTr("Done")
                onClicked:              _overlayRig.editMode = false
            }
        }
    }
}
