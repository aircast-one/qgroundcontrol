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
    id: _root

    required property var  overlayRig
    required property var  rcControls
    required property bool hasVehicle
    required property var  send            // function(channel, pwm)
    required property string ghostHint

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

    Repeater {
        model: _root._sliders

        delegate: ArrangeableOverlayItem {
            id:                 rcSliderSlot
            required property int index
            required property var modelData
            overlayRig:         _root.overlayRig
            control:            rcSlider
            editKey:            "rcSlider" + modelData.channel
            settingsKeyPrefix:  "RcControl-slider" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root.width - _root._margins - rcSliderSlot.width - index * (rcSliderSlot.width + _root._stackGap)
            defaultY:           (_root.height - rcSliderSlot.height) / 2

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
            }
        }
    }

    Repeater {
        model: _root._toggles

        delegate: ArrangeableOverlayItem {
            id:                 rcButtonSlot
            required property int index
            required property var modelData
            overlayRig:         _root.overlayRig
            control:            rcButtonColumn
            editKey:            "rcButton" + modelData.channel
            settingsKeyPrefix:  "RcControl-button" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root._margins + index * (rcButtonSlot.width + _root._stackGap)
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
            }
        }
    }

    Repeater {
        model: _root._switches3

        delegate: ArrangeableOverlayItem {
            id:                 rcSwitch3Slot
            required property int index
            required property var modelData
            overlayRig:         _root.overlayRig
            control:            rcSwitch3Column
            editKey:            "rcSwitch3" + modelData.channel
            settingsKeyPrefix:  "RcControl-switch3" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root.width - _root._margins - rcSwitch3Slot.width - index * (rcSwitch3Slot.width + _root._stackGap)
            defaultY:           _root.height / 2 - _root._margins * 7 - rcSwitch3Slot.height

            Column {
                id:      rcSwitch3Column
                spacing: ScreenTools.defaultFontPixelHeight * 0.15

                OverlaySegmentedControl {
                    id:         rcSwitch3
                    objectName: "rcControlSwitch3" + rcSwitch3Slot.modelData.channel
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
            }
        }
    }

    Repeater {
        model: _root._momentaries

        delegate: ArrangeableOverlayItem {
            id:                 rcMomentarySlot
            required property int index
            required property var modelData
            overlayRig:         _root.overlayRig
            control:            rcMomentaryColumn
            editKey:            "rcMomentary" + modelData.channel
            settingsKeyPrefix:  "RcControl-momentary" + modelData.channel
            hint:               _root.ghostHint
            defaultX:           _root._margins + index * (rcMomentarySlot.width + _root._stackGap)
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
            }
        }
    }
}
