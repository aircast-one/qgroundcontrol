/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.AutoPilotPlugin
import MAVLink

//-------------------------------------------------------------------------
//-- Battery Indicator
Item {
    id:             control
    anchors.top:    parent.top
    anchors.bottom: parent.bottom
    width:          batteryIndicatorRow.width

    property bool       showIndicator:      true
    property bool       waitForParameters:  false   // UI won't show until parameters are ready
    property Component  expandedPageComponent

    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property var    _batterySettings:   QGroundControl.settingsManager.batteryIndicatorSettings
    property Fact   _indicatorDisplay:  _batterySettings.valueDisplay
    property bool   _showPercentage:    _indicatorDisplay.rawValue === 0
    property bool   _showVoltage:       _indicatorDisplay.rawValue === 1
    property bool   _showBoth:          _indicatorDisplay.rawValue === 2

    function totalPowerText() {
        if (!_activeVehicle || _activeVehicle.batteries.count === 0) {
            return ""
        }
        const batteries = Array.from({ length: _activeVehicle.batteries.count },
                                     (_, i) => _activeVehicle.batteries.get(i))
        const watts = batteries.map(battery => battery.instantPower.rawValue)
        const amps  = batteries.map(battery => battery.current.rawValue)
        if (watts.every(value => !isNaN(value))) {
            return Math.round(watts.reduce((sum, value) => sum + value, 0)) + qsTr("W")
        }
        if (amps.every(value => !isNaN(value))) {
            return amps.reduce((sum, value) => sum + value, 0).toFixed(1) + qsTr("A")
        }
        return ""
    }

    // Properties to hold the thresholds
    property int threshold1: _batterySettings.threshold1.rawValue
    property int threshold2: _batterySettings.threshold2.rawValue   

    Row {
        id:             batteryIndicatorRow
        anchors.top:    parent.top
        anchors.bottom: parent.bottom
        spacing:        ScreenTools.defaultFontPixelWidth * 1.5

        Repeater {
            model: _activeVehicle ? _activeVehicle.batteries : 0

            Loader {
                anchors.top:        parent.top
                anchors.bottom:     parent.bottom
                sourceComponent:    batteryVisual

                property var battery:      object
                property int batteryIndex: index
            }
        }

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            font.pointSize:         ScreenTools.mediumFontPointSize
            color:                  qgcPal.toolbarText
            text:                   totalPowerText()
            visible:                text !== ""
        }
    }
    MouseArea {
        anchors.fill:   parent
        onClicked: {
            mainWindow.showIndicatorDrawer(batteryPopup, control)
        }
    }

    Component {
        id: batteryPopup

        ToolIndicatorPage {
            showExpand:         expandedComponent ? true : false
            waitForParameters:  control.waitForParameters
            contentComponent:   batteryContentComponent
            expandedComponent:  batteryExpandedComponent
        }
    }

    Component {
        id: batteryVisual

        Row {
            anchors.top:    parent.top
            anchors.bottom: parent.bottom
            spacing:        ScreenTools.defaultFontPixelWidth * 0.6

            function getBatteryColor() {
                switch (battery.chargeState.rawValue) {
                    case MAVLink.MAV_BATTERY_CHARGE_STATE_OK:
                        if (!isNaN(battery.percentRemaining.rawValue)) {
                            if (battery.percentRemaining.rawValue > threshold1) {
                                return qgcPal.toolbarText 
                            } else if (battery.percentRemaining.rawValue > threshold2) {
                                return qgcPal.colorYellowGreen 
                            } else {
                                return qgcPal.colorYellow 
                            }
                        } else {
                            return qgcPal.toolbarText
                        }
                    case MAVLink.MAV_BATTERY_CHARGE_STATE_LOW:
                        return qgcPal.colorOrange
                    case MAVLink.MAV_BATTERY_CHARGE_STATE_CRITICAL:
                    case MAVLink.MAV_BATTERY_CHARGE_STATE_EMERGENCY:
                    case MAVLink.MAV_BATTERY_CHARGE_STATE_FAILED:
                    case MAVLink.MAV_BATTERY_CHARGE_STATE_UNHEALTHY:
                        return qgcPal.colorRed
                    default:
                        return qgcPal.toolbarText
                }
            }    

            function getBatteryPercentageText() {
                if (!isNaN(battery.percentRemaining.rawValue)) {
                    if (battery.percentRemaining.rawValue > 98.9) {
                        return qsTr("100%")
                    } else {
                        return battery.percentRemaining.valueString + battery.percentRemaining.units
                    }
                } else if (!isNaN(battery.voltage.rawValue)) {
                    return battery.voltage.valueString + battery.voltage.units
                } else if (battery.chargeState.rawValue !== MAVLink.MAV_BATTERY_CHARGE_STATE_UNDEFINED) {
                    return battery.chargeState.enumStringValue
                }
                return qsTr("n/a")
            }

            function getBatteryVoltageText() {
                if (!isNaN(battery.timeRemaining.rawValue)) {
                    return battery.timeRemainingStr.rawValue
                } else if (!isNaN(battery.voltage.rawValue)) {
                    return battery.voltage.valueString + battery.voltage.units
                } else if (battery.chargeState.rawValue !== MAVLink.MAV_BATTERY_CHARGE_STATE_UNDEFINED) {
                    return battery.chargeState.enumStringValue
                }
                return qsTr("n/a")
            }

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                text:                   qsTr("B%1").arg(batteryIndex + 1)
                font.pointSize:         ScreenTools.smallFontPointSize
                color:                  qgcPal.toolbarText
                visible:                _activeVehicle && _activeVehicle.batteries.count > 1
            }

            Item {
                id:                     batteryGlyph
                anchors.verticalCenter: parent.verticalCenter
                height:                 ScreenTools.defaultFontPixelHeight * 1.15
                width:                  height * 2

                readonly property real _fill: isNaN(battery.percentRemaining.rawValue) ? 0
                                                : Math.max(0, Math.min(1, battery.percentRemaining.rawValue / 100))

                Rectangle {
                    id:             shell
                    anchors.left:   parent.left
                    width:          parent.width - nub.width
                    height:         parent.height
                    radius:         height * 0.3
                    color:          "transparent"
                    border.color:   getBatteryColor()
                    border.width:   Math.max(1, parent.height * 0.09)

                    Rectangle {
                        anchors.left:           parent.left
                        anchors.leftMargin:     shell.border.width * 2
                        anchors.verticalCenter: parent.verticalCenter
                        height:                 parent.height - shell.border.width * 4
                        width:                  (parent.width - shell.border.width * 4) * batteryGlyph._fill
                        radius:                 height * 0.25
                        color:                  getBatteryColor()
                    }
                }

                Rectangle {
                    id:                     nub
                    anchors.left:           shell.right
                    anchors.verticalCenter: parent.verticalCenter
                    width:                  parent.height * 0.14
                    height:                 parent.height * 0.42
                    radius:                 width / 2
                    color:                  getBatteryColor()
                }
            }

           ColumnLayout {
                id:                     batteryInfoColumn
                anchors.top:            parent.top
                anchors.bottom:         parent.bottom
                spacing:                0

                QGCLabel {
                    Layout.alignment:       Qt.AlignHCenter
                    verticalAlignment:      Text.AlignVCenter
                    color:                  qgcPal.toolbarText
                    text:                   getBatteryPercentageText()
                    font.pointSize:         _showBoth ? ScreenTools.defaultFontPointSize : ScreenTools.mediumFontPointSize
                    visible:                _showBoth || _showPercentage
                }

                QGCLabel {
                    Layout.alignment:       Qt.AlignHCenter
                    font.pointSize:         _showBoth ? ScreenTools.defaultFontPointSize : ScreenTools.mediumFontPointSize
                    color:                  qgcPal.toolbarText
                    text:                   getBatteryVoltageText()
                    visible:                _showBoth || _showVoltage
                }

            }
        }
    }

    Component {
        id: batteryContentComponent

        ColumnLayout {
            spacing: ScreenTools.defaultFontPixelHeight / 2

            readonly property var _batteries: _activeVehicle ? _activeVehicle.batteries : null
            readonly property int _batteryCount: _batteries ? _batteries.count : 0

            function _severityOf(chargeState) {
                switch (chargeState) {
                case MAVLink.MAV_BATTERY_CHARGE_STATE_EMERGENCY:
                case MAVLink.MAV_BATTERY_CHARGE_STATE_FAILED:
                case MAVLink.MAV_BATTERY_CHARGE_STATE_UNHEALTHY:
                    return 3
                case MAVLink.MAV_BATTERY_CHARGE_STATE_CRITICAL:
                    return 2
                case MAVLink.MAV_BATTERY_CHARGE_STATE_LOW:
                    return 1
                default:
                    return 0
                }
            }

            // The pack in the worst shape decides what the header says: a healthy second
            // battery must not soften the one that is about to bring the aircraft down.
            readonly property var _worst: {
                if (_batteryCount === 0) {
                    return null
                }
                const packs = Array.from({ length: _batteryCount }, (_, i) => _batteries.get(i))
                return packs.reduce((worst, pack) =>
                    _severityOf(pack.chargeState.rawValue) > _severityOf(worst.chargeState.rawValue) ? pack : worst)
            }

            readonly property color _worstColor: {
                if (!_worst) {
                    return qgcPal.text
                }
                switch (_severityOf(_worst.chargeState.rawValue)) {
                case 3:
                case 2:  return qgcPal.colorRed
                case 1:  return qgcPal.colorOrange
                default: return qgcPal.text
                }
            }

            function _duration(seconds) {
                if (isNaN(seconds) || seconds < 0) {
                    return ""
                }
                const total   = Math.round(seconds)
                const hours   = Math.floor(total / 3600)
                const minutes = Math.floor((total % 3600) / 60)
                const secs    = total % 60
                const pad     = (n) => n < 10 ? "0" + n : "" + n
                if (total < 60) {
                    return qsTr("%1 sec").arg(total)
                }
                return hours > 0 ? hours + ":" + pad(minutes) + ":" + pad(secs)
                                 : minutes + ":" + pad(secs)
            }

            function _packColor(pack) {
                switch (_severityOf(pack.chargeState.rawValue)) {
                case 3:
                case 2:  return qgcPal.colorRed
                case 1:  return qgcPal.colorOrange
                default: return qgcPal.colorGrey
                }
            }

            function _valueOr(fact, suffix) {
                return isNaN(fact.rawValue) ? qsTr("\u2014") : fact.valueString + suffix
            }

            // The answer first: what state the aircraft is in and how long it has. The table
            // below is the evidence for it, not the headline.
            // Rows are label-left/value-right, so the panel needs a floor to size against or
            // the two columns collide on short readings.
            Item {
                Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 32
                Layout.preferredHeight: 0
            }

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            0
                visible:            _worst !== null

                readonly property bool _alarming: _worst ? _severityOf(_worst.chargeState.rawValue) > 0 : false

                QGCLabel {
                    // Nothing wrong: the charge is the answer. Something wrong: the fault is,
                    // and the charge steps down to the supporting line.
                    text: {
                        if (!_worst) {
                            return ""
                        }
                        return parent._alarming || isNaN(_worst.percentRemaining.rawValue)
                                   ? _worst.chargeState.enumStringValue
                                   : Math.round(_worst.percentRemaining.rawValue) + qsTr("%")
                    }
                    color:          _worstColor
                    font.bold:      true
                    font.features:  { "tnum": 1 }
                    font.pointSize: ScreenTools.largeFontPointSize
                }

                QGCLabel {
                    text: {
                        if (!_worst) {
                            return ""
                        }
                        const left    = _duration(_worst.timeRemaining.rawValue)
                        const percent = isNaN(_worst.percentRemaining.rawValue)
                                            ? "" : Math.round(_worst.percentRemaining.rawValue) + qsTr("%")
                        const parts   = [ left ? qsTr("%1 left").arg(left) : "" ]
                        if (parent._alarming) {
                            parts.push(percent)
                        }
                        return parts.filter(part => part !== "").join(qsTr("  ·  "))
                    }
                    color:          qgcPal.colorGrey
                    font.features:  { "tnum": 1 }
                    visible:        text !== ""
                }
            }

            // A panel that announces an emergency and offers nothing to do about it makes the
            // pilot go hunting for the control. Only shown when the vehicle can actually
            // return, and it routes through the normal slide-to-confirm.
            OverlayMenuItem {
                Layout.fillWidth:   true
                text:               qsTr("Return")
                textColor:          qgcPal.colorRed
                visible:            _worst !== null && _severityOf(_worst.chargeState.rawValue) >= 2 &&
                                        globals.guidedControllerFlyView && globals.guidedControllerFlyView.showRTL

                onClicked: {
                    mainWindow.closeIndicatorDrawer()
                    globals.guidedControllerFlyView.confirmAction(globals.guidedControllerFlyView.actionRTL)
                }
            }

            Repeater {
                model: _batteries

                SettingsGroupLayout {
                    Layout.fillWidth:   true
                    heading:            _batteryCount === 1 ? "" : qsTr("Battery %1").arg(index + 1)
                    contentSpacing:     0
                    // Apple's grouped list: hairlines between rows, no box drawn around them.
                    showDividers:       true
                    showBorder:         false

                    property var batteryValuesAvailable: batteryValuesAvailableLoader.item

                    Loader {
                        id:                 batteryValuesAvailableLoader
                        sourceComponent:    batteryValuesAvailableComponent

                        property var battery: object
                    }

                    LabelledLabel {
                        label:          qsTr("Time left")
                        labelText:      _duration(object.timeRemaining.rawValue)
                        labelTextColor: _packColor(object)
                        visible:        batteryValuesAvailable.timeRemainingAvailable
                    }

                    LabelledLabel {
                        label:          qsTr("Charge")
                        labelText:      Math.round(object.percentRemaining.rawValue) + qsTr("%")
                        labelTextColor: _packColor(object)
                        visible:        batteryValuesAvailable.percentRemainingAvailable
                    }

                    LabelledLabel {
                        fontPointSize: ScreenTools.smallFontPointSize
                        label:          qsTr("Voltage")
                        labelText:      _valueOr(object.voltage, qsTr(" V"))
                        labelTextColor: qgcPal.colorGrey
                        // Guarded like every other row: an unavailable reading is an absent
                        // row, not a "--.-- v" that reads as a broken instrument.
                        visible:        !isNaN(object.voltage.rawValue)
                    }

                    LabelledLabel {
                        fontPointSize: ScreenTools.smallFontPointSize
                        label:          qsTr("Consumed")
                        labelText:      _valueOr(object.mahConsumed, qsTr(" mAh"))
                        labelTextColor: qgcPal.colorGrey
                        visible:        batteryValuesAvailable.mahConsumedAvailable
                    }

                    LabelledLabel {
                        fontPointSize: ScreenTools.smallFontPointSize
                        label:          qsTr("Temperature")
                        labelText:      _valueOr(object.temperature, qsTr("\u00B0C"))
                        labelTextColor: qgcPal.colorGrey
                        visible:        batteryValuesAvailable.temperatureAvailable
                    }

                    LabelledLabel {
                        fontPointSize: ScreenTools.smallFontPointSize
                        label:          qsTr("Function")
                        labelText:      object.function.enumStringValue
                        labelTextColor: qgcPal.colorGrey
                        visible:        batteryValuesAvailable.showFunction
                    }
                }
            }

            Component {
                id: batteryValuesAvailableComponent

                QtObject {
                    property bool functionAvailable:         battery.function.rawValue !== MAVLink.MAV_BATTERY_FUNCTION_UNKNOWN
                    property bool showFunction:              functionAvailable && battery.function.rawValue != MAVLink.MAV_BATTERY_FUNCTION_ALL
                    property bool temperatureAvailable:      !isNaN(battery.temperature.rawValue)
                    property bool currentAvailable:          !isNaN(battery.current.rawValue)
                    property bool mahConsumedAvailable:      !isNaN(battery.mahConsumed.rawValue)
                    property bool timeRemainingAvailable:    !isNaN(battery.timeRemaining.rawValue)
                    property bool percentRemainingAvailable: !isNaN(battery.percentRemaining.rawValue)
                    property bool chargeStateAvailable:      battery.chargeState.rawValue !== MAVLink.MAV_BATTERY_CHARGE_STATE_UNDEFINED
                }
            }
        }
    }

    Component {
        id: batteryExpandedComponent

        ColumnLayout {
            spacing: ScreenTools.defaultFontPixelHeight / 2

            FactPanelController { id: controller }

            SettingsGroupLayout {
                heading:            qsTr("Battery Display")
                Layout.fillWidth:   true

                LabelledFactComboBox {
                    id:             editModeCheckBox
                    label:          qsTr("Value")
                    fact:           _fact
                    visible:        _fact,visible

                    property Fact _fact: QGroundControl.settingsManager.batteryIndicatorSettings.valueDisplay
                }

                ColumnLayout {
                    QGCLabel { text: qsTr("Coloring") }

                    RowLayout {
                        spacing: ScreenTools.defaultFontPixelWidth * 0.05  // Reduced spacing between elements

                        // Battery 100%
                        RowLayout {
                            spacing: ScreenTools.defaultFontPixelWidth * 0.05  // Tighter spacing for icon and label
                            QGCColoredImage {
                                source: "/qmlimages/BatteryGreen.svg"
                                width: ScreenTools.defaultFontPixelWidth * 6
                                height: width
                                fillMode: Image.PreserveAspectFit
                                color: qgcPal.colorGreen
                            }
                            QGCLabel { text: qsTr("100%") }
                        }

                        // Threshold 1
                        RowLayout {
                            spacing: ScreenTools.defaultFontPixelWidth * 0.05  // Tighter spacing for icon and field
                            QGCColoredImage {
                                source: "/qmlimages/BatteryYellowGreen.svg"
                                width: ScreenTools.defaultFontPixelWidth * 6
                                height: width
                                fillMode: Image.PreserveAspectFit
                                color: qgcPal.colorYellowGreen
                            }
                            FactTextField {
                                id: threshold1Field
                                fact: _batterySettings.threshold1
                                implicitWidth: ScreenTools.defaultFontPixelWidth * 6
                                height: ScreenTools.defaultFontPixelHeight * 1.5
                                enabled: fact.visible
                                onEditingFinished: {
                                    // Validate and set the new threshold value
                                    _batterySettings.setThreshold1(parseInt(text));
                                }
                            }
                        }

                        // Threshold 2
                        RowLayout {
                            spacing: ScreenTools.defaultFontPixelWidth * 0.05  // Tighter spacing for icon and field
                            QGCColoredImage {
                                source: "/qmlimages/BatteryYellow.svg"
                                width: ScreenTools.defaultFontPixelWidth * 6
                                height: width
                                fillMode: Image.PreserveAspectFit
                                color: qgcPal.colorYellow
                            }
                            FactTextField {
                                fact: _batterySettings.threshold2
                                implicitWidth: ScreenTools.defaultFontPixelWidth * 6
                                height: ScreenTools.defaultFontPixelHeight * 1.5
                                enabled: fact.visible
                                onEditingFinished: {
                                    // Validate and set the new threshold value
                                    _batterySettings.setThreshold2(parseInt(text));                                
                                }
                            }
                        }

                        // Low state
                        RowLayout {
                            spacing: ScreenTools.defaultFontPixelWidth * 0.05  // Tighter spacing for icon and label
                            QGCColoredImage {
                                source: "/qmlimages/BatteryOrange.svg"
                                width: ScreenTools.defaultFontPixelWidth * 6
                                height: width
                                fillMode: Image.PreserveAspectFit
                                color: qgcPal.colorOrange
                            }
                            QGCLabel { text: qsTr("Low") }
                        }

                        // Critical state
                        RowLayout {
                            spacing: ScreenTools.defaultFontPixelWidth * 0.05  // Tighter spacing for icon and label
                            QGCColoredImage {
                                source: "/qmlimages/BatteryCritical.svg"
                                width: ScreenTools.defaultFontPixelWidth * 6
                                height: width
                                fillMode: Image.PreserveAspectFit
                                color: qgcPal.colorRed
                            }
                            QGCLabel { text: qsTr("Critical") }
                        }
                    }
                }
            }

            Loader {
                Layout.fillWidth: true
                sourceComponent: expandedPageComponent
            }

            SettingsGroupLayout {
                visible: _activeVehicle.autopilotPlugin.knownVehicleComponentAvailable(AutoPilotPlugin.KnownPowerVehicleComponent) &&
                            QGroundControl.corePlugin.showAdvancedUI

                LabelledButton {
                    label:      qsTr("Vehicle Power")
                    buttonText: qsTr("Configure")

                    onClicked: {
                        mainWindow.showKnownVehicleComponentConfigPage(AutoPilotPlugin.KnownPowerVehicleComponent)
                        mainWindow.closeIndicatorDrawer()
                    }
                }                
            }
        }
    }
}
