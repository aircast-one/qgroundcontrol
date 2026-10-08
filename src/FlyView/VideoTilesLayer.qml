pragma ComponentBehavior: Bound

import QtQuick
import QtMultimedia
import QtQuick.Effects

import QGroundControl
import QGroundControl.Controls

Item {
    id: _root

    property Item pipView
    property var  overlayRig: null
    property real topInset:   0
    property bool tucked:     QGroundControl.loadBoolGlobalSetting(_tuckedSettingsKey, false)
    property bool grid:       QGroundControl.loadBoolGlobalSetting(_gridSettingsKey, false)

    readonly property bool showing:          visible && _cameraNumber > 0
    readonly property real dockExtent:       showing ? Math.max(0, dock.x + dock.width - dock._pipRight + _gap) : 0
    readonly property real pipWidthOverride: grid && showing ? _cellWidth : 0

    readonly property string _tuckedSettingsKey: "VideoRailTucked"
    readonly property string _gridSettingsKey:   "VideoRailGrid"
    readonly property real   _gap:           ScreenTools.defaultFontPixelHeight / 2
    readonly property real   _capsuleHeight: ScreenTools.defaultFontPixelHeight * 1.8
    readonly property real   _radius:        ScreenTools.defaultFontPixelHeight * 0.5
    readonly property bool   _editMode:      overlayRig ? overlayRig.editMode : false
    readonly property var    _qgcPal:        QGroundControl.globalPalette

    readonly property int _cameraNumber: QGroundControl.videoManager.pipCameraNumber

    readonly property real _pipNaturalWidth: pipView ? pipView.naturalWidth : ScreenTools.defaultFontPixelWidth * 40
    readonly property real _cellWidth:   pipView ? Math.floor(Math.min(_pipNaturalWidth,
                                                                       (_root.width - pipView.x - _gap * 2 - buttons.width) / 2,
                                                                       (_root.height - topInset - _gap * 3) * 16 / 9))
                                                 : _pipNaturalWidth
    readonly property real _cellHeight:  Math.round(_cellWidth * 9 / 16)
    readonly property real _thumbWidth:  Math.round(_pipNaturalWidth * 0.36)
    readonly property real _thumbHeight: Math.round(_thumbWidth * 9 / 16)

    visible: QGroundControl.videoManager.isStreamSource && !QGroundControl.videoManager.fullScreen &&
             pipView && pipView.visible && pipView.expanded

    Component.onCompleted:   if (overlayRig) overlayRig.registerStatic(dock, pipView)
    Component.onDestruction: if (overlayRig) overlayRig.unregisterStatic(dock)

    function setTucked(value) {
        QGroundControl.saveBoolGlobalSetting(_tuckedSettingsKey, value)
        tucked = value
    }

    function setGrid(value) {
        QGroundControl.saveBoolGlobalSetting(_gridSettingsKey, value)
        grid = value
    }

    Item {
        id:         dock
        objectName: "videoRail"
        visible:    _root._cameraNumber > 0
        width:      rail.width + buttons.width
        height:     Math.max(rail.height, buttons.height)

        readonly property real _pipRight: _root.pipView ? _root.pipView.x + _root.pipView.width : 0
        readonly property real _span:     _root._gap + _root._thumbWidth + buttons.width
        readonly property bool onLeft:    !_root.grid && _root.pipView ? _pipRight + _span > _root.width && _root.pipView.x - _span >= 0 : false

        x: _root.pipView ? (_root.grid ? _root.pipView.x : onLeft ? _root.pipView.x - _root._gap - width : _pipRight + _root._gap) : _root._gap
        y: _root.pipView ? _root.pipView.y + _root.pipView.height - height : _root.height - height - _root._gap

        Item {
            id:             rail
            objectName:     "videoRailStrip"
            x:              dock.onLeft ? buttons.width : 0
            width:          _root.tucked && !_root.grid ? 0 : content.width
            height:         content.height
            anchors.bottom: parent.bottom
            clip:           true

            Behavior on width { NumberAnimation { duration: 320; easing.type: Easing.OutCubic } }

            Item {
                id:             content
                objectName:     "videoRailColumn"
                width:          _root.grid ? _root._cellWidth * 2 + _root._gap : _root._thumbWidth
                height:         tile.height
                x:              dock.onLeft ? 0 : rail.width - width
                anchors.bottom: parent.bottom

                Rectangle {
                    id:           tile
                    objectName:   "videoTile"
                    x:            _root.grid ? _root._cellWidth + _root._gap : 0
                    width:        _root.grid ? _root._cellWidth : _root._thumbWidth
                    height:       _root.grid ? _root._cellHeight : noSignal ? _root._capsuleHeight : _root._thumbHeight
                    radius:       _root._radius
                    color:        noSignal ? "transparent" : "black"
                    border.width: noSignal ? 0 : 1
                    border.color: tileMouseArea.containsMouse ? Qt.alpha(_root._qgcPal.overlayInk, 0.6)
                                                              : _root._qgcPal.overlayBorder
                    visible:      _root._cameraNumber > 0
                    layer.enabled: true
                    layer.effect:  OverlayShadowEffect { elevated: false }

                    OverlayGlass {
                        anchors.fill: parent
                        visible:      tile.noSignal
                        radius:       parent.radius
                        highlight:    tileMouseArea.containsMouse
                    }

                    readonly property int    cameraNumber: _root._cameraNumber
                    readonly property string cameraSignal: QGroundControl.videoManager.cameraSignals[cameraNumber - 1] ?? "idle"
                    readonly property bool   live:         cameraSignal === "live"
                    readonly property bool   connecting:   cameraSignal === "connecting"
                    readonly property bool   noSignal:     !live && !connecting
                    readonly property bool   recording: {
                        const flags = QGroundControl.videoManager.cameraRecording
                        const cam = cameraNumber - 1
                        return cam >= 0 && cam < flags.length ? flags[cam] : false
                    }
                    readonly property color dotColor: live ? _root._qgcPal.colorGreen : connecting ? _root._qgcPal.colorOrange : _root._qgcPal.colorRed

                    Behavior on x      { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                    Behavior on width  { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                    Behavior on height { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }

                    VideoOutput {
                        id:             tileVideo
                        objectName:     "videoTileVideo"
                        anchors.fill:   parent
                        fillMode:       VideoOutput.PreserveAspectFit
                        visible:        !tile.noSignal
                        layer.enabled:  true
                        layer.effect:   MultiEffect {
                            maskEnabled:        true
                            maskSource:         tileVideoMask
                            maskThresholdMin:   0.5
                            maskSpreadAtMin:    1.0
                        }
                        Component.onCompleted: QGroundControl.videoManager.registerPipItem(this)
                    }

                    Item {
                        id:             tileVideoMask
                        anchors.fill:   tileVideo
                        layer.enabled:  true
                        visible:        false

                        Rectangle {
                            anchors.fill:   parent
                            radius:         tile.radius
                            color:          "black"
                        }
                    }

                    QGCSpinner {
                        anchors.centerIn:   parent
                        visible:            tile.connecting
                    }

                    Rectangle {
                        id:      nameChip
                        x:       _root._gap / 2
                        y:       tile.noSignal && !_root.grid ? (tile.height - height) / 2 : _root._gap / 2
                        width:   chipRow.width + ScreenTools.defaultFontPixelWidth * 1.5
                        height:  chipRow.height + ScreenTools.defaultFontPixelHeight * 0.4
                        radius:  height / 2
                        color:   tile.noSignal ? "transparent" : Qt.rgba(0, 0, 0, 0.5)

                        Row {
                            id:                 chipRow
                            anchors.centerIn:   parent
                            spacing:            ScreenTools.defaultFontPixelWidth * 0.6

                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                width:                  ScreenTools.defaultFontPixelHeight * 0.4
                                height:                 width
                                radius:                 width / 2
                                color:                  tile.dotColor
                            }

                            QGCLabel {
                                anchors.verticalCenter: parent.verticalCenter
                                text:                   tile.cameraNumber > 0 ? QGroundControl.videoManager.cameraName(tile.cameraNumber - 1) : ""
                                color:                  "white"
                                font.pointSize:         ScreenTools.smallFontPointSize
                                font.bold:              true
                                elide:                  Text.ElideRight
                                width:                  Math.min(implicitWidth, tile.width - ScreenTools.defaultFontPixelWidth * 5)
                            }
                        }
                    }

                    Rectangle {
                        id:      recDot
                        anchors.right:   parent.right
                        anchors.top:     parent.top
                        anchors.margins: _root._gap / 2 + ScreenTools.defaultFontPixelHeight * 0.2
                        width:   ScreenTools.defaultFontPixelHeight * 0.5
                        height:  width
                        radius:  width / 2
                        color:   _root._qgcPal.colorRed
                        visible: tile.recording && !tile.noSignal

                        SequentialAnimation on opacity {
                            running: recDot.visible
                            loops:   Animation.Infinite
                            NumberAnimation { to: 0.3; duration: 500 }
                            NumberAnimation { to: 1.0; duration: 500 }
                        }
                    }

                    MouseArea {
                        id:              tileMouseArea
                        anchors.fill:    parent
                        acceptedButtons: Qt.LeftButton | Qt.RightButton
                        hoverEnabled:    true
                        cursorShape:     Qt.PointingHandCursor
                        onClicked: (mouse) => {
                            if (mouse.button === Qt.RightButton) {
                                if (_root.overlayRig) _root.overlayRig.editMode = !_root.overlayRig.editMode
                            } else if (!_root._editMode) {
                                QGroundControl.videoManager.promotePip()
                            }
                        }
                        onPressAndHold: if (_root.overlayRig) _root.overlayRig.hold(tileMouseArea)
                    }
                }
            }
        }

        Column {
            id:         buttons
            x:          dock.onLeft ? 0 : rail.width
            y:          (dock.height - height) / 2
            spacing:    _root._gap / 2

            OverlayRoundButton {
                id:          layoutButton
                objectName:  "videoRailLayout"
                icon:        "/InstrumentValueIcons/view-tile.svg"
                checked:     _root.grid
                editing:     _root._editMode
                onClicked:   _root.setGrid(!_root.grid)
                onHeld:      if (_root.overlayRig) _root.overlayRig.hold(layoutButton)
            }

            OverlayRoundButton {
                id:           tabButton
                objectName:   "videoRailTab"
                anchors.horizontalCenter: parent.horizontalCenter
                width:        layoutButton.width * 0.5
                aspect:       2.2
                icon:         "/InstrumentValueIcons/cheveron-left.svg"
                iconRotation: (_root.tucked !== dock.onLeft) ? 180 : 0
                visible:      !_root.grid
                editing:      _root._editMode
                onClicked:    _root.setTucked(!_root.tucked)
                onHeld:       if (_root.overlayRig) _root.overlayRig.hold(tabButton)
            }
        }
    }
}
