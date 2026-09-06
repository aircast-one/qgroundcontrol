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
import Viewer3D

Item {
    id: _root
    property var planController:    _planController
    property var guidedController:  _guidedController
    property var overlayRig:        _overlayRig

    OverlayRig {
        id:         _overlayRig
        objectName: "overlayRigFlyView"
        viewport:   _root
        topInset:   toolbar.height
    }

    Item {
        objectName: "overlayDropPreview"
        x:          _overlayRig.dropPreview.x
        y:          _overlayRig.dropPreview.y
        width:      _overlayRig.dropPreview.width
        height:     _overlayRig.dropPreview.height
        z:          QGroundControl.zOrderWidgets - 1
        visible:    opacity > 0
        opacity:    _overlayRig.dropPreviewVisible ? 1 : 0

        OverlayGlass {
            anchors.fill: parent
            radius:       _overlayRig.dropPreviewRadius
            highlight:    true
        }

        ShaderEffectSource {
            anchors.fill:  parent
            sourceItem:    _overlayRig.dropPreviewVisible ? _overlayRig.draggedItem : null
            live:          true
            opacity:       0.55
            layer.enabled: true
            layer.effect:  MultiEffect {
                blurEnabled: true
                blur:        0.6
                blurMax:     24
                saturation:  -0.6
            }
        }

        Behavior on x       { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        Behavior on y       { NumberAnimation { duration: 120; easing.type: Easing.OutCubic } }
        Behavior on opacity { NumberAnimation { duration: 150 } }
    }

    Repeater {
        model: _overlayRig.spacingReadouts

        Rectangle {
            required property var modelData
            objectName: "overlaySpacingReadout"
            x:          modelData.x - width / 2
            y:          modelData.y - height / 2
            width:      readoutLabel.implicitWidth + ScreenTools.defaultFontPixelHeight * 0.7
            height:     ScreenTools.defaultFontPixelHeight * 1.1
            radius:     height / 2
            z:          QGroundControl.zOrderWidgets + 2
            color:      Qt.alpha(QGroundControl.globalPalette.window, 0.9)

            QGCLabel {
                id:               readoutLabel
                anchors.centerIn: parent
                text:             parent.modelData.text
                font.pointSize:   ScreenTools.smallFontPointSize
                font.bold:        true
            }
        }
    }

    Rectangle {
        objectName: "overlayGuideX"
        x:          isNaN(_overlayRig.guideX) ? 0 : Math.round(_overlayRig.guideX)
        y:          0
        width:      1
        height:     parent.height
        z:          QGroundControl.zOrderWidgets + 1
        color:      Qt.alpha(QGroundControl.globalPalette.text, 0.75)
        visible:    opacity > 0
        opacity:    !isNaN(_overlayRig.guideX) ? 1 : 0

        Behavior on opacity { NumberAnimation { duration: 100 } }
    }

    Rectangle {
        objectName: "overlayGuideY"
        x:          0
        y:          isNaN(_overlayRig.guideY) ? 0 : Math.round(_overlayRig.guideY)
        width:      parent.width
        height:     1
        z:          QGroundControl.zOrderWidgets + 1
        color:      Qt.alpha(QGroundControl.globalPalette.text, 0.75)
        visible:    opacity > 0
        opacity:    !isNaN(_overlayRig.guideY) ? 1 : 0

        Behavior on opacity { NumberAnimation { duration: 100 } }
    }

    Shortcut {
        sequence:       "Escape"
        enabled:        _overlayRig.editMode
        onActivated:    _overlayRig.editMode = false
    }
    property bool utmspSendActTrigger: false

    PlanMasterController {
        id:                     _planController
        flyView:                true
        Component.onCompleted:  start()
    }

    property bool   _mainWindowIsMap:       mapControl.pipState.state === mapControl.pipState.fullState
    property bool   _isFullWindowItemDark:  _mainWindowIsMap ? mapControl.isSatelliteMap : true
    Binding {
        target:     OverlayBackdrop
        property:   "isDark"
        value:      _root._isFullWindowItemDark
    }
    Binding {
        target:     OverlayBackdrop
        property:   "sourceAnimating"
        value:      QGroundControl.videoManager.decoding ||
                        QGroundControl.multiVehicleManager.activeVehicle !== null ||
                        _overlayRig.editMode
    }
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

    readonly property alias mapControl: mapControl

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
        topEdgeLeftInset:       toolbar.height
        topEdgeCenterInset:     toolbar.height
        topEdgeRightInset:      toolbar.height
    }

    FlyViewToolBar {
        id:                 toolbar
        z:                  QGroundControl.zOrderWidgets
        visible:            !QGroundControl.videoManager.fullScreen && mainWindow.flyViewActive
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
                visible:    mainWindow.flyViewActive
            }
        }

        PipView {
            id:                     _pipView
            objectName:             "pipView"
            fullContentItem:        backdropContent
            margin:                 _toolsMargin
            item1IsFullSettingsKey: "MainFlyWindowIsMap"
            forceItem1Full:         !mainWindow.flyViewActive
            item1:                  mapControl
            item2:                  QGroundControl.videoManager.hasVideo ? videoControl : null
            show:                   mainWindow.flyViewActive && QGroundControl.videoManager.hasVideo && !QGroundControl.videoManager.fullScreen &&
                                        (videoControl.pipState.state === videoControl.pipState.pipState || mapControl.pipState.state === mapControl.pipState.pipState)
            z:                      QGroundControl.zOrderWidgets
            showActionButton:       QGroundControl.videoManager.hasMultipleVideoSources &&
                                        videoControl.pipState.state === videoControl.pipState.pipState
            actionButtonText:       QGroundControl.videoManager.cameraName(QGroundControl.videoManager.activeVideoSource)
            onActionButtonClicked:  QGroundControl.videoManager.switchActiveVideoSource()

            widthOverride:          videoTilesLayer.pipWidthOverride

            property real leftEdgeBottomInset: visible && expanded && !hasCustomPosition ? width + videoTilesLayer.dockExtent + _toolsMargin : 0
            property real bottomEdgeLeftInset: visible && expanded && !hasCustomPosition ? height + _toolsMargin : 0

            overlayRig:            _overlayRig
        }

        FlyViewWidgetLayer {
            id:                     widgetLayer
            overlayRig:             _overlayRig
            anchors.top:            parent.top
            anchors.bottom:         parent.bottom
            anchors.left:           parent.left
            anchors.right:          guidedValueSlider.visible ? guidedValueSlider.left : parent.right
            z:                      _fullItemZorder + 2
            parentToolInsets:       _toolInsets
            mapControl:             _mapControl
            visible:                !QGroundControl.videoManager.fullScreen && mainWindow.flyViewActive
            utmspActTrigger:        utmspSendActTrigger
            isViewer3DOpen:         viewer3DWindow.isOpen
        }

        FlyViewCustomLayer {
            id:                 customOverlay
            anchors.fill:       widgetLayer
            z:                  _fullItemZorder + 2
            parentToolInsets:   widgetLayer.totalToolInsets
            mapControl:         _mapControl
            visible:            !QGroundControl.videoManager.fullScreen && mainWindow.flyViewActive
        }

        CameraControlLayer {
            id:                 cameraControlLayer
            objectName:         "cameraControlLayer"
            overlayRig:         _overlayRig
            anchors.fill:       parent
            visible:            mainWindow.flyViewActive
            videoIsMainItem:    !_mainWindowIsMap
            z:                  _fullItemZorder + 1
        }

        VideoTilesLayer {
            id:             videoTilesLayer
            objectName:     "videoTilesLayer"
            visible:        mainWindow.flyViewActive
            overlayRig:     _overlayRig
            pipView:        _pipView
            topInset:       toolbar.height
            anchors.fill:   parent
            z:              _fullItemZorder + 3
        }
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
        leftInset:      widgetLayer.totalToolInsets.leftEdgeBottomInset
        rightInset:     widgetLayer.totalToolInsets.rightEdgeBottomInset
        anchors.fill:   parent
        visible:        toolbar.visible
        z:              QGroundControl.zOrderWidgets
    }
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
