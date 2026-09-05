/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Effects

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlightDisplay
import QGroundControl.ScreenTools

Item {
    id: _root

    property Item pipView
    property var  overlayRig: null
    property real topInset:   0
    property bool tucked:     QGroundControl.loadBoolGlobalSetting(_tuckedSettingsKey, false)
    property bool grid:       QGroundControl.loadBoolGlobalSetting(_gridSettingsKey, false)

    readonly property bool multiCamera:      _tileCount > 0 && QGroundControl.videoManager.isStreamSource && !QGroundControl.videoManager.fullScreen
    readonly property bool showing:          visible && _tileCount > 0
    readonly property real dockExtent:       showing ? Math.max(0, dock.x + dock.width - dock._pipRight + _gap) : 0
    readonly property real pipWidthOverride: grid && showing ? _cellWidth : 0
    readonly property var  cameraNumbers:    [QGroundControl.videoManager.activeVideoSource + 1, ..._visibleSlots.map((entry) => entry.cam)]

    readonly property string _tuckedSettingsKey: "VideoRailTucked"
    readonly property string _gridSettingsKey:   "VideoRailGrid"
    readonly property int    _maxShown:      3
    readonly property real   _gap:           ScreenTools.defaultFontPixelHeight / 2
    readonly property real   _capsuleHeight: ScreenTools.defaultFontPixelHeight * 1.8
    readonly property real   _radius:        ScreenTools.defaultFontPixelHeight * 0.5
    readonly property bool   _editMode:      overlayRig ? overlayRig.editMode : false
    readonly property var    _qgcPal:        QGroundControl.globalPalette
    readonly property int    _maxTiles:      QGroundControl.videoManager.maxVideoTiles()

    readonly property var _slots: (QGroundControl.videoManager.activeVideoSource,
                                   QGroundControl.settingsManager.videoSettings.multiViewEnabled.rawValue,
                                   Array.from({ length: _maxTiles }, (_, slot) => QGroundControl.videoManager.tileCameraNumber(slot)))
    readonly property var _visibleSlots: _slots.map((cam, slot) => ({ cam, slot })).filter((entry) => entry.cam > 0)
    readonly property int _tileCount:    _visibleSlots.length
    readonly property int _overflow:     grid ? 0 : Math.max(0, _tileCount - _maxShown)
    readonly property int _cols:         _tileCount + 1 <= 4 ? 2 : 3
    readonly property int _rows:         Math.ceil((_tileCount + 1) / _cols)

    readonly property real _pipNaturalWidth: pipView ? pipView.naturalWidth : ScreenTools.defaultFontPixelWidth * 40
    readonly property real _cellWidth:   pipView ? Math.floor(Math.min(_pipNaturalWidth,
                                                                       (_root.width - pipView.x - _gap * _cols - buttons.width) / _cols,
                                                                       (_root.height - topInset - _gap * 3 - _gap * (_rows - 1)) / _rows * 16 / 9))
                                                 : _pipNaturalWidth
    readonly property real _cellHeight:  Math.round(_cellWidth * 9 / 16)
    readonly property real _thumbWidth:  grid ? _cellWidth : Math.round(_pipNaturalWidth * 0.36)
    readonly property real _thumbHeight: Math.round(_thumbWidth * 9 / 16)

    readonly property var _shown:   grid ? _visibleSlots : _visibleSlots.slice(0, _maxShown)
    readonly property var _heights: (QGroundControl.videoManager.cameraStatuses, QGroundControl.videoManager.cameraConnecting,
                                     _shown.map((entry) => grid ? _cellHeight : (stateOf(entry.cam) === "nosignal" ? _capsuleHeight : _thumbHeight)))
    readonly property var _tops:    _heights.map((_, i) => _heights.slice(0, i).reduce((sum, h) => sum + h + _gap, 0))
    readonly property var _placement: _slots.map((cam, slot) => {
        const i = _shown.findIndex((entry) => entry.slot === slot)
        if (i < 0) {
            return { shown: false, x: 0, y: 0, w: _thumbWidth, h: _thumbHeight }
        }
        if (grid) {
            const cell = i + 1
            const col = cell % _cols
            const row = Math.floor(cell / _cols)
            return { shown: true, x: col * (_cellWidth + _gap), y: (_rows - 1 - row) * (_cellHeight + _gap), w: _cellWidth, h: _cellHeight }
        }
        return { shown: true, x: 0, y: _tops[i], w: _thumbWidth, h: _heights[i] }
    })
    readonly property real _contentWidth:  grid ? _cols * _cellWidth + (_cols - 1) * _gap : _thumbWidth
    readonly property real _contentHeight: grid ? _rows * _cellHeight + (_rows - 1) * _gap
                                                : _heights.reduce((sum, h) => sum + h + _gap, 0) - (_heights.length ? _gap : 0) + (_overflow > 0 ? _gap + _capsuleHeight : 0)

    visible: QGroundControl.videoManager.isStreamSource && !QGroundControl.videoManager.fullScreen &&
             pipView && pipView.visible && pipView.expanded

    function stateOf(cameraNumber) {
        const statuses = QGroundControl.videoManager.cameraStatuses
        const flags = QGroundControl.videoManager.cameraConnecting
        const cam = cameraNumber - 1
        const status = cam < 0 ? "" : cam < statuses.length ? statuses[cam] : qsTr("No signal")
        const connecting = cam >= 0 && cam < flags.length ? flags[cam] : false
        return status === "" ? "live" : connecting ? "connecting" : "nosignal"
    }

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
        visible:    _root._tileCount > 0
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
            width:          _root.tucked && !_root.grid ? 0 : _root._contentWidth
            height:         content.height
            anchors.bottom: parent.bottom
            clip:           true

            Behavior on width { NumberAnimation { duration: 320; easing.type: Easing.OutCubic } }

            Item {
                id:             content
                objectName:     "videoRailColumn"
                width:          _root._contentWidth
                height:         _root._contentHeight
                x:              dock.onLeft ? 0 : rail.width - width
                anchors.bottom: parent.bottom

                Repeater {
                    model: _root._maxTiles

                    delegate: Rectangle {
                        id:           tile
                        objectName:   "videoTile" + index
                        x:            place.x
                        y:            place.y
                        width:        place.w
                        height:       place.h
                        radius:       _root._radius
                        color:        noSignal ? "transparent" : "black"
                        border.width: noSignal ? 0 : 1
                        border.color: tileMouseArea.containsMouse ? Qt.alpha(_root._qgcPal.text, 0.6)
                                                                  : _root._qgcPal.overlayBorder
                        visible:      place.shown
                        layer.enabled: true
                        layer.effect:  OverlayShadowEffect { elevated: false }

                        OverlayGlass {
                            anchors.fill: parent
                            visible:      tile.noSignal
                            radius:       parent.radius
                            highlight:    tileMouseArea.containsMouse
                        }

                        readonly property var    place:        _root._placement[index]
                        readonly property int    cameraNumber: _root._slots[index]
                        readonly property string state:        (QGroundControl.videoManager.cameraStatuses, QGroundControl.videoManager.cameraConnecting, _root.stateOf(cameraNumber))
                        readonly property bool   live:         state === "live"
                        readonly property bool   connecting:   state === "connecting"
                        readonly property bool   noSignal:     state === "nosignal"
                        readonly property bool   recording: {
                            const flags = QGroundControl.videoManager.cameraRecording
                            const cam = cameraNumber - 1
                            return cam >= 0 && cam < flags.length ? flags[cam] : false
                        }
                        readonly property color dotColor: live ? _root._qgcPal.colorGreen : connecting ? _root._qgcPal.colorOrange : _root._qgcPal.colorRed

                        Behavior on x      { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                        Behavior on y      { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                        Behavior on width  { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                        Behavior on height { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }

                        QGCVideoBackground {
                            id:             tileVideo
                            objectName:     "extraVideo" + index
                            anchors.fill:   parent
                            visible:        !tile.noSignal
                            layer.enabled:  true
                            layer.effect:   MultiEffect {
                                maskEnabled:        true
                                maskSource:         tileVideoMask
                                maskThresholdMin:   0.5
                                maskSpreadAtMin:    1.0
                            }
                            Component.onCompleted: QGroundControl.videoManager.registerTileItem(index, this)
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
                                    QGroundControl.videoManager.promoteTile(index)
                                }
                            }
                            onPressAndHold: if (_root.overlayRig) _root.overlayRig.editMode = true
                        }
                    }
                }

                Rectangle {
                    id:           more
                    objectName:   "videoTileMore"
                    y:            _root._contentHeight - height
                    width:        _root._thumbWidth
                    height:       _root._capsuleHeight
                    radius:       _root._radius
                    color:        "transparent"
                    visible:      _root._overflow > 0

                    OverlayGlass {
                        anchors.fill: parent
                        radius:       parent.radius
                        highlight:    moreMouseArea.containsMouse
                    }

                    QGCLabel {
                        anchors.centerIn: parent
                        text:             qsTr("+%1 more").arg(_root._overflow)
                        color:            "white"
                        font.pointSize:   ScreenTools.smallFontPointSize
                        font.bold:        true
                    }

                    MouseArea {
                        id:           moreMouseArea
                        anchors.fill: parent
                        hoverEnabled: true
                        cursorShape:  Qt.PointingHandCursor
                        onClicked:    _root.setGrid(true)
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
                onHeld:      if (_root.overlayRig) _root.overlayRig.editMode = true
            }

            OverlayRoundButton {
                objectName:   "videoRailTab"
                anchors.horizontalCenter: parent.horizontalCenter
                width:        layoutButton.width * 0.5
                aspect:       2.2
                icon:         "/InstrumentValueIcons/cheveron-left.svg"
                iconRotation: (_root.tucked !== dock.onLeft) ? 180 : 0
                visible:      !_root.grid
                editing:      _root._editMode
                onClicked:    _root.setTucked(!_root.tucked)
                onHeld:       if (_root.overlayRig) _root.overlayRig.editMode = true
            }
        }
    }
}
