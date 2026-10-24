import QtQuick
import QtQuick.Shapes
import QtPositioning

import QGroundControl
import QGroundControl.Controls

Item {
    id: _root

    required property var overlayRig

    property Item pipView
    property Item pipState: videoPipState

    property int    _track_rec_x:       0
    property int    _track_rec_y:       0

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

    // Leaving the fly view tears the stream down rather than hiding it. Hiding the item leaves
    // the receiver decoding, holding its socket and burning the GPU behind a panel nobody is
    // looking at - there is no video to plan with, so there is no video to run.
    Connections {
        target: mainWindow

        function onFlyViewActiveChanged() {
            if (mainWindow.flyViewActive) {
                videoStartDelay.start()
            } else {
                videoStartDelay.stop()
                QGroundControl.videoManager.stopVideo()
            }
        }
    }

    //-- Video Streaming
    FlightDisplayViewVideo {
        id:             videoStreaming
        anchors.fill:   parent
        useSmallFont:   _root.pipState.state !== _root.pipState.fullState
        visible:        QGroundControl.videoManager.isStreamSource
    }

    // The "no video" pill carries Retry and Video Settings, so a telemetry chip parked on top
    // of it takes away controls the user needs exactly when the video is broken. Owned by the
    // pip because the pill sits inside the video, and the video is sometimes the pip itself -
    // without the owner the rig would push the pip away from its own contents.
    Component.onCompleted:   _root.overlayRig.registerStatic(videoStreaming.statusPill, _root.pipView)
    Component.onDestruction: _root.overlayRig.unregisterStatic(videoStreaming.statusPill)
    //-- UVC Video (USB Camera or Video Device)
    Loader {
        id:             cameraLoader
        anchors.fill:   parent
        visible:        QGroundControl.videoManager.isUvc
        source:         QGroundControl.videoManager.uvcEnabled ? "qrc:/qml/QGroundControl/FlyView/FlightDisplayViewUVC.qml" : "qrc:/qml/QGroundControl/FlyView//FlightDisplayViewDummy.qml"
    }

    QGCLabel {
        text: qsTr("Double-click to exit full screen")
        font.pointSize: ScreenTools.largeFontPointSize
        visible: QGroundControl.videoManager.fullScreen && flyViewVideoMouseArea.containsMouse
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
        screenX:                 flyViewVideoMouseArea.mouseX
        screenY:                 flyViewVideoMouseArea.mouseY
        cameraTrackingEnabled:   videoStreaming._camera && videoStreaming._camera.trackingEnabled
    }

    // Only while there is a picture to interact with. This fills the whole video area for
    // gimbal click-control and tracking-ROI drawing, and it is declared after the video, so it
    // sits on top of everything the video draws - including the no-signal state's Retry and
    // Video Settings buttons, which it swallowed. Click-to-aim and ROI selection both need a
    // frame to act on, so there is nothing for it to do when nothing is decoding.
    MouseArea {
        id:                         flyViewVideoMouseArea
        anchors.fill:               parent
        enabled:                    pipState.state === pipState.fullState &&
                                        QGroundControl.videoManager.decoding
        hoverEnabled:               true

        property double x0:         0
        property double x1:         0
        property double y0:         0
        property double y1:         0
        property double offset_x:   0
        property double offset_y:   0
        property double radius:     20
        property var trackingROI:   null
        property var trackingStatus: trackingStatusComponent.createObject(flyViewVideoMouseArea, {})
        property bool _dragging:    false
        property bool _longPressed: false
        readonly property real _dragThreshold: 10

        onClicked:       onScreenGimbalController.clickControl()
        onDoubleClicked: QGroundControl.videoManager.fullScreen = !QGroundControl.videoManager.fullScreen

        onPressed:(mouse) => {
            onScreenGimbalController.pressControl()

            _track_rec_x = mouse.x
            _track_rec_y = mouse.y
            _dragging = false
            _longPressed = false
            if (_activeVehicle) {
                holdRing.start(mouse.x, mouse.y)
            }

            //create a new rectangle at the wanted position
            if(videoStreaming._camera) {
                if (videoStreaming._camera.trackingEnabled) {
                    trackingROI = trackingROIComponent.createObject(flyViewVideoMouseArea, {
                        "x": mouse.x,
                        "y": mouse.y
                    });
                }
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
            if (!_dragging && (Math.abs(mouse.x - _track_rec_x) >= _dragThreshold || Math.abs(mouse.y - _track_rec_y) >= _dragThreshold)) {
                _dragging = true
                holdRing.stop()
            }
            if (trackingROI !== null) {
                if (mouse.x < trackingROI.x) {
                    trackingROI.x = mouse.x
                    trackingROI.width = Math.abs(mouse.x - _track_rec_x)
                } else {
                    trackingROI.width = Math.abs(mouse.x - trackingROI.x)
                }
                if (mouse.y < trackingROI.y) {
                    trackingROI.y = mouse.y
                    trackingROI.height = Math.abs(mouse.y - _track_rec_y)
                } else {
                    trackingROI.height = Math.abs(mouse.y - trackingROI.y)
                }
            }
        }
        onReleased: (mouse) => {
            onScreenGimbalController.releaseControl()
            holdRing.stop()

            //if there is already a selection, delete it
            if (trackingROI !== null) {
                trackingROI.destroy();
            }
            if (_longPressed) {
                _longPressed = false
                return
            }

            if(videoStreaming._camera) {
                if (videoStreaming._camera.trackingEnabled) {
                    // order coordinates --> top/left and bottom/right
                    x0 = Math.min(_track_rec_x, mouse.x)
                    x1 = Math.max(_track_rec_x, mouse.x)
                    y0 = Math.min(_track_rec_y, mouse.y)
                    y1 = Math.max(_track_rec_y, mouse.y)

                    //calculate offset between video stream rect and background (black stripes)
                    offset_x = (parent.width - videoStreaming.getWidth()) / 2
                    offset_y = (parent.height - videoStreaming.getHeight()) / 2

                    //convert absolute coords in background to absolute video stream coords
                    x0 = x0 - offset_x
                    x1 = x1 - offset_x
                    y0 = y0 - offset_y
                    y1 = y1 - offset_y

                    //convert absolute to relative coordinates and limit range to 0...1
                    x0 = Math.max(Math.min(x0 / videoStreaming.getWidth(), 1.0), 0.0)
                    x1 = Math.max(Math.min(x1 / videoStreaming.getWidth(), 1.0), 0.0)
                    y0 = Math.max(Math.min(y0 / videoStreaming.getHeight(), 1.0), 0.0)
                    y1 = Math.max(Math.min(y1 / videoStreaming.getHeight(), 1.0), 0.0)

                    //use point message if rectangle is very small
                    if (Math.abs(_track_rec_x - mouse.x) < 10 && Math.abs(_track_rec_y - mouse.y) < 10) {
                        var pt  = Qt.point(x0, y0)
                        videoStreaming._camera.startTrackingPoint(pt, radius / videoStreaming.getWidth())
                    } else {
                        var rec = Qt.rect(x0, y0, x1 - x0, y1 - y0)
                        videoStreaming._camera.startTrackingRect(rec)
                    }
                    _track_rec_x = 0
                    _track_rec_y = 0
                }
            }
        }

        Component {
            id: trackingROIComponent

            Rectangle {
                color:              Qt.rgba(0.1,0.85,0.1,0.25)
                border.color:       "green"
                border.width:       1
            }
        }

        Component {
            id: trackingStatusComponent

            Rectangle {
                color:              "transparent"
                border.color:       "red"
                border.width:       5
                radius:             5
            }
        }

        Timer {
            id: trackingStatusTimer
            interval:               50
            repeat:                 true
            running:                true
            onTriggered: {
                if (videoStreaming._camera) {
                    if (videoStreaming._camera.trackingEnabled && videoStreaming._camera.trackingImageStatus) {
                        var margin_hor = (parent.parent.width - videoStreaming.getWidth()) / 2
                        var margin_ver = (parent.parent.height - videoStreaming.getHeight()) / 2
                        var left = margin_hor + videoStreaming.getWidth() * videoStreaming._camera.trackingImageRect.left
                        var top = margin_ver + videoStreaming.getHeight() * videoStreaming._camera.trackingImageRect.top
                        var right = margin_hor + videoStreaming.getWidth() * videoStreaming._camera.trackingImageRect.right
                        var bottom = margin_ver + !isNaN(videoStreaming._camera.trackingImageRect.bottom) ? videoStreaming.getHeight() * videoStreaming._camera.trackingImageRect.bottom : top + (right - left)
                        var width = right - left
                        var height = bottom - top

                        flyViewVideoMouseArea.trackingStatus.x = left
                        flyViewVideoMouseArea.trackingStatus.y = top
                        flyViewVideoMouseArea.trackingStatus.width = width
                        flyViewVideoMouseArea.trackingStatus.height = height
                    } else {
                        flyViewVideoMouseArea.trackingStatus.x = 0
                        flyViewVideoMouseArea.trackingStatus.y = 0
                        flyViewVideoMouseArea.trackingStatus.width = 0
                        flyViewVideoMouseArea.trackingStatus.height = 0
                    }
                }
            }
        }
    }

    //-- Note: the camera-switch button for the full-screen video is provided by FlyView at a
    //   higher z so it is not hidden behind the instrument overlays. The small-pip button is
    //   provided by PipView.

    ProximityRadarVideoView{
        anchors.fill:   parent
        vehicle:        QGroundControl.multiVehicleManager.activeVehicle
    }

    ObstacleDistanceOverlayVideo {
        id: obstacleDistance
        showText: pipState.state === pipState.fullState
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

        property var  _pendingCircle:   globals && globals.guidedControllerFlyView ? globals.guidedControllerFlyView.orbitMapCircle : null
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
