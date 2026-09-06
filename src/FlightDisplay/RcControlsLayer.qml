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

// Renders the user-defined on-screen controls from Settings > Fly View > Custom RC Controls.
// CameraControlLayer owns the built-in camera/gimbal chrome and hands this layer the already
// deduped, already-reserved-channel-filtered list to draw.
Item {
    id: _root

    required property var  overlayRig
    required property var  rcControls
    required property bool hasVehicle
    required property var  send            // function(channel, pwm)
    required property var  rotate          // function(channel) - flips a slider's orientation
    required property string ghostHint

    property real rightInset: 0

    property real _sliderWidth: 0

    readonly property real _sliderColumn: _sliderWidth + _stackGap
    readonly property int  _leftSliders:  Math.floor(_sliders.length / 2)
    readonly property real _buttonInset:  _leftSliders * _sliderColumn

    readonly property int _pwmMin:    1000
    readonly property int _pwmMax:    2000
    readonly property int _pwmCenter: 1500

    property real _margins:  ScreenTools.defaultFontPixelHeight
    readonly property real _stackGap: _margins / 2

    readonly property var _switch3Pwms: [_pwmMin, _pwmCenter, _pwmMax]

    readonly property var _sliders:     rcControls.filter((control) => control.type === "slider")
    readonly property var _toggles:     rcControls.filter((control) => control.type === "button")
    readonly property var _switches3:   rcControls.filter((control) => control.type === "switch3")
    readonly property var _momentaries: rcControls.filter((control) => control.type === "momentary")

    // The Settings page stamps the channel it just added into this global setting. Pick it up
    // the moment the control list changes (i.e. as soon as this layer exists to show it) and
    // clear it so the pulse plays once, not every time the user revisits the fly view.
    property int _justAddedChannel: -1

    function _checkJustAdded() {
        const justAdded = QGroundControl.loadGlobalSetting("rcControlJustAdded", "")
        if (justAdded !== "") {
            QGroundControl.saveGlobalSetting("rcControlJustAdded", "")
            _justAddedChannel = parseInt(justAdded)
            _clearJustAdded.restart()
        }
    }

    // onRcControlsChanged alone misses the case where the control was added and the app was
    // restarted before this layer ever existed to see the change - the flag would otherwise
    // sit there and wrongly highlight whatever channel happens to change next.
    Component.onCompleted: _checkJustAdded()
    onRcControlsChanged:   _checkJustAdded()

    Timer { id: _clearJustAdded; interval: 1500; onTriggered: _root._justAddedChannel = -1 }

    // Long-press is the only way to arrange, hide, or rotate a control, and nothing on this
    // screen says so - the Settings page description is the only place that's written down.
    // Nudge once per fly view visit until the user actually discovers it for themselves.
    property bool _editModeDiscovered: QGroundControl.loadBoolGlobalSetting("rcControlEditModeDiscovered", false)

    Connections {
        target: overlayRig
        function onEditModeChanged() {
            if (overlayRig.editMode && !_root._editModeDiscovered) {
                _root._editModeDiscovered = true
                QGroundControl.saveBoolGlobalSetting("rcControlEditModeDiscovered", true)
            }
        }
    }

    Item {
        id:      _longPressHint
        visible: opacity > 0
        // _autoFaded is a plain flag rather than the Timer assigning opacity directly: writing
        // to a bound property snaps the binding off for good, so the hint could never come back
        // once conditions change (e.g. the fly view is revisited before discovery happens).
        property bool _autoFaded: false
        opacity: (_root.rcControls.length > 0 && !_root._editModeDiscovered && !overlayRig.editMode && !_autoFaded) ? 1 : 0
        anchors.top:              parent.top
        anchors.topMargin:        ScreenTools.defaultFontPixelHeight * 3
        anchors.horizontalCenter: parent.horizontalCenter
        width:                    hintRow.width + ScreenTools.defaultFontPixelWidth * 4
        height:                   ScreenTools.defaultFontPixelHeight * 2.4
        z:                        QGroundControl.zOrderTopMost

        Behavior on opacity { NumberAnimation { duration: 400 } }

        Timer { interval: 6000; running: _longPressHint.opacity > 0; onTriggered: _longPressHint._autoFaded = true }

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.height / 2
        }

        Row {
            id:               hintRow
            anchors.centerIn: parent
            spacing:          ScreenTools.defaultFontPixelWidth

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                font.pointSize:         ScreenTools.smallFontPointSize
                text:                   qsTr("Long-press a control to arrange, hide, or rotate it")
            }
        }
    }

    component NewControlPulse: SequentialAnimation {
        id: _pulse
        required property Item target
        loops: 3
        NumberAnimation { target: _pulse.target; property: "scale"; from: 1; to: 1.12; duration: 220; easing.type: Easing.OutQuad }
        NumberAnimation { target: _pulse.target; property: "scale"; from: 1.12; to: 1; duration: 220; easing.type: Easing.InQuad }
    }

    Repeater {
        model: _root._sliders

        delegate: ArrangeableOverlayItem {
            id:                 rcSliderSlot
            required property int index
            required property var modelData
            rig:                _root.overlayRig
            control:            rcSlider
            editKey:            "rcSlider" + modelData.channel
            settingsKeyPrefix:  "RcControl-slider" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           index % 2 === 0
                                    ? _root.width - _root.rightInset - _root._margins - rcSliderSlot.width
                                          - Math.floor(index / 2) * (rcSliderSlot.width + _root._stackGap)
                                    : _root._margins + Math.floor(index / 2) * (rcSliderSlot.width + _root._stackGap)
            defaultY:           (_root.height - rcSliderSlot.height) / 2

            onWidthChanged:        if (index === 0) _root._sliderWidth = width
            Component.onCompleted: if (index === 0) _root._sliderWidth = width

            CameraEdgeSlider {
                id:                     rcSlider
                objectName:             "rcControlSlider" + rcSliderSlot.modelData.channel
                lifted:                 rcSliderSlot.dragging
                editing:                overlayRig.editMode
                actionsEnabled:         _root.hasVehicle
                readout:                rcSliderSlot.modelData.label || "CH" + rcSliderSlot.modelData.channel
                valueReadout:           true
                centered:               true
                vertical:               rcSliderSlot.modelData.orientation !== "horizontal"
                from:                   _root._pwmMin
                to:                     _root._pwmMax
                value:                  _root._pwmCenter
                onMoved:                (v) => { value = v; _root.send(rcSliderSlot.modelData.channel, v) }
                onRecenterRequested:    { value = _root._pwmCenter; _root.send(rcSliderSlot.modelData.channel, _root._pwmCenter) }
                onHeld:                 overlayRig.hold(rcSlider)

                Connections {
                    target: QGroundControl.multiVehicleManager
                    function onActiveVehicleChanged() { rcSlider.value = _root._pwmCenter }
                }

                NewControlPulse { target: rcSlider; running: rcSliderSlot.modelData.channel === _root._justAddedChannel }
            }

            // Mirrors OverlayEditBadge's hide/show badge on the opposite corner: edit mode is
            // also the only time a slider's orientation is worth changing.
            OverlayCapsule {
                id:         rotateBadge
                objectName: "rcControlRotate" + rcSliderSlot.modelData.channel
                width:      ScreenTools.defaultFontPixelHeight * 1.2
                height:     width
                highlight:  true
                z:          10
                visible:    overlayRig.editMode

                anchors.horizontalCenter:       parent.right
                anchors.verticalCenter:         parent.top
                anchors.horizontalCenterOffset: -width / 2
                anchors.verticalCenterOffset:   width / 2

                function activate() { _root.rotate(rcSliderSlot.modelData.channel) }

                QGCColoredImage {
                    anchors.centerIn:  parent
                    width:             parent.width * 0.6
                    height:            width
                    fillMode:          Image.PreserveAspectFit
                    sourceSize.height: height
                    color:             rotateBadge.contentColor
                    source:            "/InstrumentValueIcons/reload.svg"
                }

                QGCMouseArea {
                    anchors.fill:    parent
                    anchors.margins: -Math.max(0, (ScreenTools.minTouchPixels - parent.width) / 2)
                    onClicked:       rotateBadge.activate()
                }
            }
        }
    }

    Repeater {
        model: _root._toggles

        delegate: ArrangeableOverlayItem {
            id:                 rcButtonSlot
            required property int index
            required property var modelData
            rig:                _root.overlayRig
            control:            rcButtonColumn
            editKey:            "rcButton" + modelData.channel
            settingsKeyPrefix:  "RcControl-button" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root._margins + _root._buttonInset + index * (rcButtonSlot.width + _root._stackGap)
            defaultY:           (_root.height - rcButtonSlot.height) / 2

            Column {
                id:      rcButtonColumn
                spacing: ScreenTools.defaultFontPixelHeight * 0.15

                OverlayRoundButton {
                    id:             rcButton
                    objectName:     "rcControlButton" + rcButtonSlot.modelData.channel
                    lifted:         rcButtonSlot.dragging
                    editing:        overlayRig.editMode
                    actionsEnabled: _root.hasVehicle
                    icon:           "/InstrumentValueIcons/swap.svg"
                    onClicked: {
                        checked = !checked
                        _root.send(rcButtonSlot.modelData.channel, checked ? _root._pwmMax : _root._pwmMin)
                    }
                    onHeld:         overlayRig.hold(rcButton)

                    Connections {
                        target: QGroundControl.multiVehicleManager
                        function onActiveVehicleChanged() { rcButton.checked = false }
                    }
                }

                OverlayCaption { text: rcButtonSlot.modelData.label || "CH" + rcButtonSlot.modelData.channel }

                NewControlPulse { target: rcButtonColumn; running: rcButtonSlot.modelData.channel === _root._justAddedChannel }
            }
        }
    }

    Repeater {
        model: _root._switches3

        delegate: ArrangeableOverlayItem {
            id:                 rcSwitch3Slot
            required property int index
            required property var modelData
            rig:                _root.overlayRig
            control:            rcSwitch3Column
            editKey:            "rcSwitch3" + modelData.channel
            settingsKeyPrefix:  "RcControl-switch3" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root.width - _root.rightInset - _root._margins - rcSwitch3Slot.width - index * (rcSwitch3Slot.width + _root._stackGap)
            defaultY:           _root.height / 2 - _root._margins * 7 - rcSwitch3Slot.height

            Column {
                id:      rcSwitch3Column
                spacing: ScreenTools.defaultFontPixelHeight * 0.15

                OverlaySegmentedControl {
                    id:           rcSwitch3
                    objectName:   "rcControlSwitch3" + rcSwitch3Slot.modelData.channel
                    glass:        true
                    contentColor: QGroundControl.globalPalette.overlayInk
                    width:      ScreenTools.defaultFontPixelWidth * 12
                    enabled:    !overlayRig.editMode && _root.hasVehicle
                    segments:   ["1", "2", "3"]
                    currentIndex: 1
                    onActivated: (i) => {
                        currentIndex = i
                        _root.send(rcSwitch3Slot.modelData.channel, _root._switch3Pwms[i])
                    }

                    TapHandler {
                        onLongPressed: overlayRig.hold(rcSwitch3)
                    }

                    Connections {
                        target: QGroundControl.multiVehicleManager
                        function onActiveVehicleChanged() { rcSwitch3.currentIndex = 1 }
                    }
                }

                OverlayCaption { text: rcSwitch3Slot.modelData.label || "CH" + rcSwitch3Slot.modelData.channel }

                NewControlPulse { target: rcSwitch3Column; running: rcSwitch3Slot.modelData.channel === _root._justAddedChannel }
            }
        }
    }

    Repeater {
        model: _root._momentaries

        delegate: ArrangeableOverlayItem {
            id:                 rcMomentarySlot
            required property int index
            required property var modelData
            rig:                _root.overlayRig
            control:            rcMomentaryColumn
            editKey:            "rcMomentary" + modelData.channel
            settingsKeyPrefix:  "RcControl-momentary" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root._margins + _root._buttonInset + index * (rcMomentarySlot.width + _root._stackGap)
            defaultY:           _root.height / 2 + _root._margins * 3

            Column {
                id:      rcMomentaryColumn
                spacing: ScreenTools.defaultFontPixelHeight * 0.15

                OverlayRoundButton {
                    id:             rcMomentaryButton
                    objectName:     "rcControlMomentary" + rcMomentarySlot.modelData.channel
                    lifted:         rcMomentarySlot.dragging
                    editing:        overlayRig.editMode
                    actionsEnabled: _root.hasVehicle
                    icon:           "/InstrumentValueIcons/bolt.svg"
                    onPressed:      { checked = true;  _root.send(rcMomentarySlot.modelData.channel, _root._pwmMax) }
                    onReleased:     { checked = false; _root.send(rcMomentarySlot.modelData.channel, _root._pwmMin) }
                    onHeld:         overlayRig.hold(rcMomentaryButton)

                    function _release() {
                        if (checked) {
                            checked = false
                            _root.send(rcMomentarySlot.modelData.channel, _root._pwmMin)
                        }
                    }

                    Connections {
                        target: QGroundControl.multiVehicleManager
                        function onActiveVehicleChanged() { rcMomentaryButton._release() }
                    }

                    // Entering edit mode (triggered by this same long-press) disables this
                    // button's own MouseArea mid-press, so no onReleased fires - force the
                    // channel back down here instead of leaving it latched at max PWM.
                    Connections {
                        target: overlayRig
                        function onEditModeChanged() {
                            if (overlayRig.editMode) {
                                rcMomentaryButton._release()
                            }
                        }
                    }
                }

                OverlayCaption { text: rcMomentarySlot.modelData.label || "CH" + rcMomentarySlot.modelData.channel }

                NewControlPulse { target: rcMomentaryColumn; running: rcMomentarySlot.modelData.channel === _root._justAddedChannel }
            }
        }
    }
}
