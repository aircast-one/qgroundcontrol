import QtQuick
import QtQuick.Shapes
import QtMultimedia
import QtPositioning

import QGroundControl
import QGroundControl.Controls

Item {
    id: _root

    property Item pipView
    property Item pipState: videoPipState

    signal groundPointPressed(var coordinate, point position)

    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property bool   _videoIsMain:       pipState.state === pipState.fullState
    readonly property string _holdHintSeenKey: "VideoPressAndHoldHintSeen"
    property bool   _holdHintSeen:      QGroundControl.loadBoolGlobalSetting(_holdHintSeenKey, false)
    property bool   _holdHintEligible:  _videoIsMain && !_holdHintSeen && !!_activeVehicle && !!_activeVehicle.gimbalController.activeGimbal
    property bool   _showHoldHint:      false
    property string _holdHintText:      ""
    property real   _overlayMargins:    ScreenTools.defaultFontPixelWidth

    on_HoldHintEligibleChanged: {
        if (_holdHintEligible) {
            _flashHoldHint(qsTr("Press and hold a spot to look at it or fly there"), 10000)
        } else {
            _showHoldHint = false
        }
    }

    function _flashHoldHint(text, durationMs) {
        _holdHintText = text
        _showHoldHint = true
        holdHintTimer.interval = durationMs
        holdHintTimer.restart()
    }

    Timer {
        id:             holdHintTimer
        onTriggered:    _showHoldHint = false
    }

    function _groundCoordinateAt(vehicle, x, y) {
        var videoWidth = videoStreaming.getWidth()
        var videoHeight = videoStreaming.getHeight()
        if (videoWidth <= 0 || videoHeight <= 0) {
            return QtPositioning.coordinate()
        }
        var panPct = ((x - (width - videoWidth) / 2) / videoWidth) * 2 - 1
        var tiltPct = 1 - ((y - (height - videoHeight) / 2) / videoHeight) * 2
        if (Math.abs(panPct) > 1 || Math.abs(tiltPct) > 1) {
            return QtPositioning.coordinate()
        }
        return vehicle.gimbalController.groundCoordinateAt(panPct, tiltPct)
    }

    PipState {
        id:         videoPipState
        pipView:    _root.pipView
        isDark:     true

        onWindowAboutToOpen: {
            QGroundControl.videoManager.stopVideo()
            videoStartDelay.start()
        }

        onWindowAboutToClose: {
            QGroundControl.videoManager.stopVideo()
            videoStartDelay.start()
        }

        onStateChanged: {
            if (pipState.state !== pipState.fullState) {
                QGroundControl.videoManager.fullScreen = false
            }
        }
    }

    Timer {
        id:           videoStartDelay
        interval:     2000;
        running:      false
        repeat:       false
        onTriggered:  QGroundControl.videoManager.startVideo()
    }

    //-- Video Streaming
    FlightDisplayViewVideo {
        id:             videoStreaming
        anchors.fill:   parent
        useSmallFont:   _root.pipState.state !== _root.pipState.fullState
        visible:        QGroundControl.videoManager.isStreamSource || QGroundControl.videoManager.isUvc
    }

    QGCLabel {
        text: qsTr("Double-click to exit full screen")
        font.pointSize: ScreenTools.largeFontPointSize
        visible: QGroundControl.videoManager.fullScreen
        anchors.centerIn: parent

        onVisibleChanged: {
            if (visible) {
                labelAnimation.start()
            }
        }

        PropertyAnimation on opacity {
            id: labelAnimation
            duration: 10000
            from: 1.0
            to: 0.0
            easing.type: Easing.InExpo
        }
    }

    OnScreenGimbalController {
        id:                      onScreenGimbalController
        anchors.fill:            parent
        cameraTrackingEnabled:   !!(videoStreaming._camera && videoStreaming._camera.trackingEnabled)
    }

    OnScreenCameraTrackingController {
        id:                      cameraTrackingController
        anchors.fill:            parent
        camera:                  videoStreaming._camera
        videoWidth:              videoStreaming.getWidth()
        videoHeight:             videoStreaming.getHeight()
    }

    MouseArea {
        id:                         flyViewVideoMouseArea
        anchors.fill:               parent
        enabled:                    pipState.state === pipState.fullState

        property real _pressX:      0
        property real _pressY:      0
        property bool _dragging:    false
        property bool _doubleClicked: false
        property bool _longPressed: false
        readonly property real _dragThreshold: 10

        Timer {
            id:         singleClickTimer
            interval:   Qt.styleHints.mouseDoubleClickInterval
            repeat:     false

            property real clickX: 0
            property real clickY: 0

            onTriggered: {
                onScreenGimbalController.mouseClicked(clickX, clickY)
                cameraTrackingController.mouseClicked(clickX, clickY)
            }
        }

        onDoubleClicked: {
            _doubleClicked = true
            singleClickTimer.stop()
            QGroundControl.videoManager.fullScreen = !QGroundControl.videoManager.fullScreen
        }

        onPressed: (mouse) => {
            _pressX = mouse.x
            _pressY = mouse.y
            _dragging = false
            _doubleClicked = false
            _longPressed = false
            if (_activeVehicle) {
                holdRing.start(mouse.x, mouse.y)
            }
        }

        onPressAndHold: (mouse) => {
            holdRing.stop()
            var vehicle = QGroundControl.multiVehicleManager.activeVehicle
            if (_dragging || !vehicle) {
                return
            }
            _longPressed = true
            if (!vehicle.gimbalController.activeGimbal) {
                _root._flashHoldHint(qsTr("The vehicle reports no gimbal, so a spot in the picture can't be placed on the map"), 4000)
                return
            }
            var coordinate = _root._groundCoordinateAt(vehicle, mouse.x, mouse.y)
            if (!coordinate.isValid) {
                _root._flashHoldHint(qsTr("Press and hold on a spot below the horizon"), 4000)
                return
            }
            _root.groundPointPressed(coordinate, flyViewVideoMouseArea.mapToItem(globals.parent, mouse.x, mouse.y))
            QGroundControl.saveBoolGlobalSetting(_holdHintSeenKey, true)
            _holdHintSeen = true
            _showHoldHint = false
        }

        onPositionChanged: (mouse) => {
            if (!_dragging && (Math.abs(mouse.x - _pressX) >= _dragThreshold || Math.abs(mouse.y - _pressY) >= _dragThreshold)) {
                _dragging = true
                holdRing.stop()
                onScreenGimbalController.mouseDragStart(_pressX, _pressY)
                cameraTrackingController.mouseDragStart(_pressX, _pressY)
            }
            if (_dragging) {
                onScreenGimbalController.mouseDragPositionChanged(mouse.x, mouse.y)
                cameraTrackingController.mouseDragPositionChanged(mouse.x, mouse.y)
            }
        }

        onReleased: (mouse) => {
            holdRing.stop()
            if (_dragging) {
                onScreenGimbalController.mouseDragEnd()
                cameraTrackingController.mouseDragEnd(mouse.x, mouse.y)
            } else if (_doubleClicked) {
                _doubleClicked = false
            } else if (_longPressed) {
                _longPressed = false
            } else {
                singleClickTimer.clickX = mouse.x
                singleClickTimer.clickY = mouse.y
                singleClickTimer.restart()
            }
            _dragging = false
        }
    }

    ProximityRadarVideoView{
        anchors.fill:   parent
        vehicle:        QGroundControl.multiVehicleManager.activeVehicle
    }

    ObstacleDistanceOverlayVideo {
        id: obstacleDistance
        showText: pipState.state === pipState.fullState
    }

    //-- Additional cameras shown simultaneously as picture-in-picture tiles
    Item {
        id:             multiViewTiles
        anchors.fill:   parent
        visible:        videoStreaming.visible

        readonly property string _tileSizeSettingsKey: "VideoTileSize"
        property real _tileSize: ScreenTools.defaultFontPixelWidth * 34
        property real _minSize:  0.10
        property real _maxSize:  0.5

        Component.onCompleted: {
            var savedSize = parseFloat(QGroundControl.loadGlobalSetting(_tileSizeSettingsKey, "0"))
            if (savedSize > 0) {
                _tileSize = savedSize
            }
        }

        Repeater {
            model: QGroundControl.videoManager.maxVideoTiles()

            delegate: Rectangle {
                id:             tile
                property int cameraNumber: (QGroundControl.videoManager.activeVideoSource,
                                            QGroundControl.settingsManager.videoSettings.multiViewEnabled.rawValue,
                                            QGroundControl.videoManager.tileCameraNumber(index))
                width:          Math.min(Math.max(multiViewTiles._tileSize, multiViewTiles.width * multiViewTiles._minSize), multiViewTiles.width * multiViewTiles._maxSize)
                height:         Math.round(width * 9 / 16)
                visible:        cameraNumber > 0 && !QGroundControl.videoManager.fullScreen && pipState.state === pipState.fullState
                color:          tileExpanded ? "black" : "transparent"

                readonly property string _tileExpandedSettingsKey: "VideoTileCamera" + cameraNumber + "Expanded"
                property bool tileExpanded: QGroundControl.loadBoolGlobalSetting(_tileExpandedSettingsKey, true)

                on_TileExpandedSettingsKeyChanged: {
                    tileExpanded = QGroundControl.loadBoolGlobalSetting(_tileExpandedSettingsKey, true)
                    QGroundControl.videoManager.registerTileItem(index, tileExpanded ? tileVideo : null)
                }

                function setTileExpanded(expanded) {
                    QGroundControl.saveBoolGlobalSetting(_tileExpandedSettingsKey, expanded)
                    tileExpanded = expanded
                    QGroundControl.videoManager.registerTileItem(index, expanded ? tileVideo : null)
                }

                DragToPosition {
                    id:                 tileDragPosition
                    target:             tile
                    settingsKeyPrefix:  "VideoTile" + index
                    defaultX:           multiViewTiles.width - tile.width - ScreenTools.defaultFontPixelWidth * 2
                    defaultY:           (multiViewTiles.height - tile.height) / 2 + index * (tile.height + ScreenTools.defaultFontPixelHeight * 0.5)
                }

                DragHandler {
                    enabled: tile.tileExpanded
                    onActiveChanged: if (!active) tileDragPosition.commit()
                }

                VideoOutput {
                    id:                 tileVideo
                    objectName:         "extraVideo" + index
                    anchors.fill:       parent
                    visible:            tile.tileExpanded
                    fillMode:           VideoOutput.PreserveAspectFit
                    Component.onCompleted: QGroundControl.videoManager.registerTileItem(index, tile.tileExpanded ? this : null)
                }

                property string statusText: {
                    var statuses = QGroundControl.videoManager.cameraStatuses
                    var cam = tile.cameraNumber - 1
                    return cam >= 0 && cam < statuses.length ? statuses[cam] : ""
                }

                QGCLabel {
                    anchors.centerIn:   parent
                    text:               tile.statusText
                    color:              "white"
                    font.pointSize:     ScreenTools.smallFontPointSize
                    visible:            tile.statusText !== "" && tile.tileExpanded
                }

                Rectangle {
                    anchors.left:       parent.left
                    anchors.top:        parent.top
                    anchors.margins:    ScreenTools.defaultFontPixelHeight / 3
                    width:              tileLabel.contentWidth + ScreenTools.defaultFontPixelWidth * 2
                    height:             tileLabel.contentHeight + ScreenTools.defaultFontPixelHeight / 2
                    radius:             ScreenTools.defaultFontPixelHeight / 3
                    color:              Qt.rgba(0,0,0,0.75)
                    visible:            tileLabel.text !== "" && tile.tileExpanded

                    QGCLabel {
                        id:                 tileLabel
                        anchors.centerIn:   parent
                        text:               tile.cameraNumber > 0 ? QGroundControl.videoManager.cameraName(tile.cameraNumber - 1) : ""
                        color:              "white"
                        font.pointSize:     ScreenTools.smallFontPointSize
                    }
                }

                MouseArea {
                    id:             tileMouseArea
                    anchors.fill:   parent
                    enabled:        tile.tileExpanded
                    hoverEnabled:   true
                    onClicked:      QGroundControl.videoManager.promoteTile(index)
                }

                ResizeHandle {
                    id:          tileResizeHandle
                    target:      tile
                    enabled:     tile.tileExpanded
                    height:      ScreenTools.defaultFontPixelHeight * 2
                    iconVisible: tile.tileExpanded && (ScreenTools.isMobile || tileMouseArea.containsMouse || tileResizeHandle.pressed)
                    onResized:   (newWidth) => { multiViewTiles._tileSize = newWidth }
                    onCommitted: {
                        multiViewTiles._tileSize = tile.width
                        QGroundControl.saveGlobalSetting(multiViewTiles._tileSizeSettingsKey, tile.width.toString())
                        if (tileDragPosition.hasCustomPosition) {
                            tileDragPosition.commit()
                        } else {
                            tileDragPosition.rebind()
                        }
                    }
                }

                Image {
                    source:             "/qmlimages/pipHide.svg"
                    mipmap:             true
                    fillMode:           Image.PreserveAspectFit
                    anchors.left:       parent.left
                    anchors.bottom:     parent.bottom
                    visible:            tile.tileExpanded && (ScreenTools.isMobile || tileMouseArea.containsMouse)
                    height:             ScreenTools.defaultFontPixelHeight * 2
                    width:              height
                    sourceSize.height:  height
                    MouseArea {
                        anchors.fill:   parent
                        onClicked:      tile.setTileExpanded(false)
                    }
                }

                Rectangle {
                    anchors.left:       parent.left
                    anchors.bottom:     parent.bottom
                    height:             ScreenTools.defaultFontPixelHeight * 2
                    width:              height
                    radius:             ScreenTools.defaultFontPixelHeight / 3
                    color:              Qt.rgba(0,0,0,0.75)
                    visible:            !tile.tileExpanded
                    Image {
                        width:              parent.width * 0.75
                        height:             parent.height * 0.75
                        sourceSize.height:  height
                        source:             "/res/buttonRight.svg"
                        mipmap:             true
                        fillMode:           Image.PreserveAspectFit
                        anchors.centerIn:   parent
                    }
                    MouseArea {
                        anchors.fill:   parent
                        onClicked:      tile.setTileExpanded(true)
                    }
                }
            }
        }
    }

    Item {
        id:         holdRing
        width:      ScreenTools.defaultFontPixelHeight * 3
        height:     width
        visible:    false

        property real progress: 0
        property real _stroke:  ScreenTools.defaultFontPixelHeight / 4

        function start(x, y) {
            holdRing.x = x - (width / 2)
            holdRing.y = y - (height / 2)
            progress = 0
            holdRingDelay.restart()
        }

        function stop() {
            holdRingDelay.stop()
            holdRingFill.stop()
            visible = false
        }

        Timer {
            id:         holdRingDelay
            interval:   150
            onTriggered: {
                holdRing.visible = true
                holdRingFill.restart()
            }
        }

        NumberAnimation {
            id:         holdRingFill
            target:     holdRing
            property:   "progress"
            from:       0
            to:         1
            duration:   Math.max(Qt.styleHints.mousePressAndHoldInterval - holdRingDelay.interval, 100)
        }

        Shape {
            anchors.fill: parent

            ShapePath {
                strokeColor:    "white"
                strokeWidth:    holdRing._stroke
                fillColor:      "transparent"
                capStyle:       ShapePath.RoundCap

                PathAngleArc {
                    centerX:    holdRing.width / 2
                    centerY:    holdRing.height / 2
                    radiusX:    (holdRing.width / 2) - holdRing._stroke
                    radiusY:    radiusX
                    startAngle: -90
                    sweepAngle: 360 * holdRing.progress
                }
            }
        }
    }

    Rectangle {
        anchors.horizontalCenter:   parent.horizontalCenter
        anchors.bottom:             parent.bottom
        anchors.bottomMargin:       parent.height * 0.2
        width:                      holdHintLabel.width + (_overlayMargins * 2)
        height:                     holdHintLabel.height + _overlayMargins
        radius:                     height / 2
        color:                      Qt.rgba(0, 0, 0, 0.6)
        visible:                    _showHoldHint

        QGCLabel {
            id:                 holdHintLabel
            anchors.centerIn:   parent
            color:              "white"
            text:               _holdHintText
        }
    }

    Canvas {
        id:             orbitOutline
        anchors.fill:   parent
        visible:        _videoIsMain && !!_activeVehicle && (_orbitPending || _activeVehicle.orbitActive)

        property var  _pendingCircle:   globals.guidedControllerFlyView ? globals.guidedControllerFlyView.orbitMapCircle : null
        property bool _orbitPending:    !!_pendingCircle && _pendingCircle.visible

        onVisibleChanged: requestPaint()

        Timer {
            interval:       200
            repeat:         true
            running:        orbitOutline.visible
            onTriggered:    orbitOutline.requestPaint()
        }

        onPaint: {
            const ctx = getContext("2d")
            ctx.reset()
            if (!visible) {
                return
            }
            const center = _orbitPending ? _pendingCircle.center : _activeVehicle.orbitMapCircle.center
            const radius = _orbitPending ? _pendingCircle.radius() : _activeVehicle.orbitMapCircle.radius.rawValue
            const videoWidth = videoStreaming.getWidth()
            const videoHeight = videoStreaming.getHeight()
            const marginH = (width - videoWidth) / 2
            const marginV = (height - videoHeight) / 2
            ctx.strokeStyle = "#00e5ff"
            ctx.lineWidth = ScreenTools.defaultFontPixelHeight / 6
            ctx.beginPath()
            _activeVehicle.gimbalController.videoOutlineOfCircle(center, radius, 48).reduce((drawing, point) => {
                const inView = !isNaN(point.x) && Math.abs(point.x) < 3 && Math.abs(point.y) < 3
                if (inView) {
                    const x = marginH + ((point.x + 1) / 2) * videoWidth
                    const y = marginV + ((1 - point.y) / 2) * videoHeight
                    if (drawing) {
                        ctx.lineTo(x, y)
                    } else {
                        ctx.moveTo(x, y)
                    }
                }
                return inView
            }, false)
            ctx.stroke()
        }
    }

    Item {
        id:                 lookAtReticle
        anchors.centerIn:   parent
        width:              ScreenTools.defaultFontPixelHeight * 4
        height:             width
        visible:            _videoIsMain && !!_activeVehicle && (_activeVehicle.isROIEnabled || _activeVehicle.orbitActive)

        Rectangle {
            anchors.fill:   parent
            radius:         width / 2
            color:          "transparent"
            border.color:   "white"
            border.width:   ScreenTools.defaultFontPixelHeight / 8
        }

        Rectangle {
            anchors.centerIn:   parent
            width:              ScreenTools.defaultFontPixelHeight / 3
            height:             width
            radius:             width / 2
            color:              "white"
        }
    }

    Rectangle {
        anchors.top:                lookAtReticle.bottom
        anchors.topMargin:          _overlayMargins
        anchors.horizontalCenter:   parent.horizontalCenter
        width:                      lookAtRow.width + (_overlayMargins * 2)
        height:                     lookAtRow.height + _overlayMargins
        radius:                     _overlayMargins
        color:                      Qt.rgba(0, 0, 0, 0.6)
        visible:                    lookAtReticle.visible

        Row {
            id:                 lookAtRow
            anchors.centerIn:   parent
            spacing:            _overlayMargins

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                color:                  "white"
                text:                   _activeVehicle && _activeVehicle.orbitActive ? qsTr("Orbiting this point") : qsTr("Looking at this point")
            }

            QGCButton {
                text:       qsTr("Stop")
                onClicked:  _activeVehicle.orbitActive ? _activeVehicle.pauseVehicle() : _activeVehicle.stopGuidedModeROI()
            }
        }
    }
}
