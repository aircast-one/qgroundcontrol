/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

Item {
    id:         _root
    visible:    !QGroundControl.videoManager.fullScreen

    required property var overlayRig

    required property bool videoIsMainItem
    opacity:    _hasVehicle || overlayRig.editMode ? 1 : 0.45

    readonly property bool _hasVehicle: _activeVehicle !== null && _activeVehicle !== undefined

    property var  _activeVehicle:   QGroundControl.multiVehicleManager.activeVehicle
    property var  _flyViewSettings: QGroundControl.settingsManager.flyViewSettings

    property int  _tiltChannel:     _flyViewSettings.gimbalTiltChannel.rawValue
    property int  _panChannel:      _flyViewSettings.gimbalPanChannel.rawValue === _tiltChannel
                                        ? 0 : _flyViewSettings.gimbalPanChannel.rawValue
    property int  _zoomChannel:     _flyViewSettings.cameraZoomChannel.rawValue
    property int  _lightChannel:    _flyViewSettings.cameraLightChannel.rawValue
    property int  _recordChannel:   _flyViewSettings.cameraRecordChannel.rawValue

    function _parseRcControls(json) {
        try {
            const parsed = JSON.parse(json)
            if (Array.isArray(parsed)) {
                return parsed
            }
            console.warn("CameraControlLayer: rcControls setting is not a list, ignoring:", json)
        } catch (e) {
            console.warn("CameraControlLayer: rcControls setting is not valid JSON, ignoring:", e.message)
        }
        return []
    }
    function _firstControlPerFreeChannel(controls) {
        const reserved = [_tiltChannel, _panChannel, _zoomChannel, _lightChannel, _recordChannel]
        return controls.reduce((kept, control) =>
            reserved.includes(control.channel) || kept.some((c) => c.channel === control.channel) ? kept : [...kept, control], [])
    }
    readonly property var _rcControls: _firstControlPerFreeChannel(
        _parseRcControls(_flyViewSettings.rcControls.rawValue).filter((control) => control.channel > 0 && control.channel <= 18))
    readonly property string _ghostHint: _hasVehicle ? "" : qsTr("Connect a vehicle to use this control")

    function _rotateSlider(channel) {
        const all = _parseRcControls(_flyViewSettings.rcControls.rawValue)
        if (all.length === 0) {
            return
        }
        _flyViewSettings.rcControls.rawValue = JSON.stringify(all.map((control) =>
            control.channel === channel && control.type === "slider"
                ? Object.assign({}, control, { orientation: control.orientation === "horizontal" ? "vertical" : "horizontal" })
                : control))
    }

    readonly property var  _gimbalController: _activeVehicle ? _activeVehicle.gimbalController : null
    readonly property var  _activeGimbal:     _gimbalController ? _gimbalController.activeGimbal : null
    readonly property bool _hasGimbalManager: _activeGimbal !== null && _activeGimbal !== undefined
    readonly property bool _hasRcGimbal:      _tiltChannel > 0 || _panChannel > 0

    property bool _hasGimbal:       _hasGimbalManager || _hasRcGimbal

    readonly property real _tiltAngle: _hasGimbalManager ? _activeGimbal.absolutePitch.value : NaN

    readonly property bool _yawLocked: _hasGimbalManager && _activeGimbal.yawLock

    function toggleYawLock() {
        if (_hasGimbalManager) {
            _gimbalController.toggleGimbalYawLock(!_yawLocked)
        }
    }
    property real _margins:         ScreenTools.defaultFontPixelHeight

    readonly property int _pwmMin:      1000
    readonly property int _pwmMax:      2000
    readonly property int _pwmCenter:   1500

    property int  _tilt:    _pwmCenter
    property int  _pan:     _pwmCenter
    property int  _zoom:    _pwmCenter
    property bool _lightOn: false

    readonly property bool _canRecordStream: QGroundControl.videoManager.hasVideo &&
                                                QGroundControl.videoManager.isStreamSource
    readonly property bool hasShutter:      _canRecordStream || _recordChannel > 0

    property bool _cameraRecording: false

    readonly property bool _recording: (_canRecordStream && QGroundControl.videoManager.recording) ||
                                        (_recordChannel > 0 && _cameraRecording)

    function toggleRecording() {
        const next = !_recording
        if (_recordChannel > 0) {
            _send(_recordChannel, next ? _pwmMax : _pwmMin)
            _cameraRecording = next
        }
        if (_canRecordStream) {
            if (next) {
                QGroundControl.videoManager.startRecording()
            } else {
                QGroundControl.videoManager.stopRecording()
            }
        }
    }

    Connections {
        target: QGroundControl.multiVehicleManager
        function onActiveVehicleChanged() {
            _tilt           = _pwmCenter
            _pan            = _pwmCenter
            _zoom           = _pwmCenter
            _lightOn        = false
            _cameraRecording = false
        }
    }

    function _send(channel, pwm) {
        if (_activeVehicle && channel > 0) {
            _activeVehicle.setRcChannelOverride(channel, pwm)
        }
    }

    function _clamp(pwm) {
        return Math.max(_pwmMin, Math.min(_pwmMax, Math.round(pwm)))
    }

    function nudgeTilt(delta) {
        _tilt = _clamp(_tilt + delta)
        _send(_tiltChannel, _tilt)
    }

    function nudgePan(delta) {
        _pan = _clamp(_pan + delta)
        _send(_panChannel, _pan)
    }

    function nudgeZoom(fraction) {
        setZoom(_zoom + fraction * (_pwmMax - _pwmMin))
    }

    function setZoom(pwm) {
        _zoom = _clamp(pwm)
        _send(_zoomChannel, _zoom)
    }

    function recenterGimbal() {
        if (_hasGimbalManager) {
            _gimbalController.centerGimbal()
            return
        }
        _tilt = _pwmCenter
        _pan  = _pwmCenter
        _send(_tiltChannel, _tilt)
        _send(_panChannel, _pan)
    }

    function aim(panFraction, tiltFraction) {
        if (_hasGimbalManager) {
            _gimbalController.gimbalOnScreenControl(panFraction, tiltFraction, false, true, true)
            return
        }
        if (_panChannel > 0) {
            nudgePan(panFraction * (_pwmMax - _pwmMin))
        }
        if (_tiltChannel > 0) {
            nudgeTilt(tiltFraction * (_pwmMax - _pwmMin))
        }
    }

    CameraAimArea {
        objectName:     "cameraAimArea"
        anchors.fill:   parent
        enabled:        _hasVehicle && (_hasGimbal || _zoomChannel > 0) && _canRecordStream &&
                            videoIsMainItem && !overlayRig.editMode
        onAimed:        (panFraction, tiltFraction) => aim(panFraction, tiltFraction)
        onZoomed:       (zoomFraction) => { if (_zoomChannel > 0) nudgeZoom(zoomFraction) }
    }

    ArrangeableOverlayItem {
        id:                 tiltSlot
        rig:                _root.overlayRig
        control:            tiltSlider
        editKey:            "tiltSlider"
        settingsKeyPrefix:  "CameraTiltSlider"
        hint:               _ghostHint
        available:          _hasGimbal
        defaultX:           _margins
        defaultY:           (_root.height - tiltSlot.height) / 2

        CameraEdgeSlider {
            id:                     tiltSlider
            objectName:             "cameraTiltSlider"
            lifted:                 tiltSlot.dragging
            editing:                overlayRig.editMode
            actionsEnabled:         _hasVehicle
            icon:                   "/qmlimages/CameraTilt.svg"
            from:                   _hasGimbalManager ? -90 : _pwmMin
            to:                     _hasGimbalManager ?  30 : _pwmMax
            value:                  _hasGimbalManager ? _tiltAngle : _tilt
            readout:                _hasGimbalManager ? qsTr("%1°").arg(Math.round(_tiltAngle)) : ""
            onMoved:                (v) => {
                if (_hasGimbalManager) {
                    _gimbalController.sendPitchBodyYaw(v, _activeGimbal.bodyYaw.value, false)
                } else {
                    _tilt = v
                    _send(_tiltChannel, v)
                }
            }
            onRecenterRequested:    recenterGimbal()
            onHeld:                 overlayRig.hold(tiltSlider)
        }
    }

    readonly property real _stackGap: _margins / 2
    readonly property real _rcHeights: (zoomSlot.visible     ? zoomSlot.height     + _stackGap : 0) +
                                       (shutterSlot.visible  ? shutterSlot.height  + _stackGap : 0) +
                                       (yawModeSlot.visible  ? yawModeSlot.height  + _stackGap : 0) +
                                       (lightSlot.visible    ? lightSlot.height    + _stackGap : 0) +
                                       (recenterSlot.visible ? recenterSlot.height + _stackGap : 0) - _stackGap
    readonly property real _rcTop:   (_root.height - _rcHeights) / 2
    readonly property real _rcAxisX: _root.width - _margins - shutterSlot.width / 2
    readonly property real rightClusterReservedWidth: _rightClusterVisible
                                                          ? _margins * 2 + Math.max(zoomSlot.width, shutterSlot.width) : 0
    readonly property bool _rightClusterVisible: zoomSlot.visible || shutterSlot.visible || yawModeSlot.visible ||
                                                     lightSlot.visible || recenterSlot.visible
    readonly property real _rcY1: _rcTop
    readonly property real _rcY2: _rcY1 + (zoomSlot.visible    ? zoomSlot.height    + _stackGap : 0)
    readonly property real _rcY3: _rcY2 + (shutterSlot.visible ? shutterSlot.height + _stackGap : 0)
    readonly property real _rcY4: _rcY3 + (yawModeSlot.visible ? yawModeSlot.height + _stackGap : 0)
    readonly property real _rcY5: _rcY4 + (lightSlot.visible   ? lightSlot.height   + _stackGap : 0)

    ArrangeableOverlayItem {
        id:                 zoomSlot
        rig:                _root.overlayRig
        control:            zoomSlider
        editKey:            "zoomSlider"
        settingsKeyPrefix:  "Camera-zoomSlider"
        hint:               _ghostHint
        available:          _zoomChannel > 0
        defaultX:           _rcAxisX - zoomSlot.width / 2
        defaultY:           _rcY1

        CameraEdgeSlider {
            id:                     zoomSlider
            objectName:             "cameraZoomSlider"
            lifted:                 zoomSlot.dragging
            editing:                overlayRig.editMode
            actionsEnabled:         _hasVehicle
            icon:                   "/qmlimages/CameraZoom.svg"
            from:                   _pwmMin
            to:                     _pwmMax
            value:                  _zoom
            onMoved:                (v) => setZoom(v)
            onRecenterRequested:    setZoom(_pwmCenter)
            onHeld:                 overlayRig.hold(zoomSlider)
        }
    }

    ArrangeableOverlayItem {
        id:                 shutterSlot
        rig:                _root.overlayRig
        control:            shutterButton
        editKey:            "shutterButton"
        settingsKeyPrefix:  "Camera-shutterButton"
        hint:               _ghostHint
        available:          hasShutter
        defaultX:           _rcAxisX - shutterSlot.width / 2
        defaultY:           _rcY2

        CameraShutterButton {
            id:             shutterButton
            objectName:     "cameraShutterButton"
            lifted:         shutterSlot.dragging
            editing:        overlayRig.editMode
            actionsEnabled: _hasVehicle
            recording:      _recording
            onClicked:      toggleRecording()
            onHeld:         overlayRig.hold(shutterButton)
        }
    }

    ArrangeableOverlayItem {
        id:                 yawModeSlot
        rig:                _root.overlayRig
        control:            yawModeColumn
        editKey:            "yawModeButton"
        settingsKeyPrefix:  "Camera-yawModeButton"
        hint:               _ghostHint
        available:          _hasGimbalManager
        defaultX:           _rcAxisX - yawModeSlot.width / 2
        defaultY:           _rcY3

        Column {
            id:      yawModeColumn
            spacing: ScreenTools.defaultFontPixelHeight * 0.15

            OverlayRoundButton {
                id:             yawModeButton
                objectName:     "cameraYawModeButton"
                lifted:         yawModeSlot.dragging
                editing:        overlayRig.editMode
                actionsEnabled: _hasVehicle
                checked:        _yawLocked
                icon:           _yawLocked ? "/qmlimages/CameraYawLock.svg"
                                           : "/qmlimages/CameraYawFollow.svg"
                onClicked:      toggleYawLock()
                onHeld:         overlayRig.hold(yawModeButton)
            }

            OverlayCaption { text: _yawLocked ? qsTr("Yaw Lock") : qsTr("Yaw Free") }
        }
    }

    ArrangeableOverlayItem {
        id:                 lightSlot
        rig:                _root.overlayRig
        control:            lightColumn
        editKey:            "lightButton"
        settingsKeyPrefix:  "Camera-lightButton"
        hint:               _ghostHint
        available:          _lightChannel > 0
        defaultX:           _rcAxisX - lightSlot.width / 2
        defaultY:           _rcY4

        Column {
            id:      lightColumn
            spacing: ScreenTools.defaultFontPixelHeight * 0.15

            OverlayRoundButton {
                id:             lightButton
                objectName:     "cameraLightButton"
                lifted:         lightSlot.dragging
                editing:        overlayRig.editMode
                actionsEnabled: _hasVehicle
                icon:           "/qmlimages/CameraLight.svg"
                checked:        _lightOn
                onClicked: {
                    _lightOn = !_lightOn
                    _send(_lightChannel, _lightOn ? _pwmMax : _pwmMin)
                }
                onHeld:         overlayRig.hold(lightButton)
            }

            OverlayCaption { text: qsTr("Light") }
        }
    }

    ArrangeableOverlayItem {
        id:                 recenterSlot
        rig:                _root.overlayRig
        control:            recenterColumn
        editKey:            "recenterButton"
        settingsKeyPrefix:  "Camera-recenterButton"
        hint:               _ghostHint
        available:          _hasGimbal
        defaultX:           _rcAxisX - recenterSlot.width / 2
        defaultY:           _rcY5

        Column {
            id:      recenterColumn
            spacing: ScreenTools.defaultFontPixelHeight * 0.15

            OverlayRoundButton {
                id:             recenterButton
                objectName:     "cameraRecenterButton"
                lifted:         recenterSlot.dragging
                editing:        overlayRig.editMode
                actionsEnabled: _hasVehicle
                icon:           "/qmlimages/CameraRecenter.svg"
                onClicked:      recenterGimbal()
                onHeld:         overlayRig.hold(recenterButton)
            }

            OverlayCaption { text: qsTr("Recenter") }
        }
    }

    RcControlsLayer {
        objectName:   "rcControlsLayer"
        anchors.fill: parent
        rightInset:   _root.rightClusterReservedWidth
        overlayRig:   _root.overlayRig
        rcControls:   _root._rcControls
        hasVehicle:   _hasVehicle
        send:         _send
        rotate:       _rotateSlider
        ghostHint:    _ghostHint
    }
}
