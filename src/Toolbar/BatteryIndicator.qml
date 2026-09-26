import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls

Item {
    id:             control
    anchors.top:    parent.top
    anchors.bottom: parent.bottom
    width:          batteryIndicatorRow.width

    property bool       showIndicator:      true

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
            showExpand:                         true
            waitForParameters:                  false
            expandedComponentWaitForParameters: true
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
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_OK:
                        return qgcPal.toolbarText
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_UNDEFINED:
                        if (!isNaN(battery.percentRemaining.rawValue)) {
                            if (battery.percentRemaining.rawValue > threshold1) {
                                return qgcPal.toolbarText
                            } else if (battery.percentRemaining.rawValue > threshold2) {
                                return qgcPal.colorYellow
                            } else {
                                return qgcPal.colorOrange
                            }
                        }
                        return qgcPal.toolbarText
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_LOW:
                        return qgcPal.colorOrange
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_CRITICAL:
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_EMERGENCY:
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_FAILED:
                    case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_UNHEALTHY:
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
                } else if (battery.chargeState.rawValue !== MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_UNDEFINED) {
                    return battery.chargeState.enumStringValue
                }
                return qsTr("n/a")
            }

            function getBatteryVoltageText() {
                if (!isNaN(battery.timeRemaining.rawValue)) {
                    return battery.timeRemainingStr.rawValue
                } else if (!isNaN(battery.voltage.rawValue)) {
                    return battery.voltage.valueString + battery.voltage.units
                } else if (battery.chargeState.rawValue !== MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_UNDEFINED) {
                    return battery.chargeState.enumStringValue
                }
                return qsTr("n/a")
            }

            QGCLabel {
                anchors.verticalCenter: parent.verticalCenter
                text:                   qsTr("B%1").arg(batteryIndex + 1)
                font.pointSize:         ScreenTools.smallFontPointSize
                color:                  qgcPal.colorGrey
                visible:                _activeVehicle && _activeVehicle.batteries.count > 1
            }

            BatteryGlyph {
                anchors.verticalCenter: parent.verticalCenter
                fill:                   isNaN(battery.percentRemaining.rawValue) ? 0
                                            : battery.percentRemaining.rawValue / 100
                color:                  getBatteryColor()
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
                    font.pointSize:         ScreenTools.mediumFontPointSize
                    visible:                _showBoth || _showPercentage
                }

                QGCLabel {
                    Layout.alignment:       Qt.AlignHCenter
                    color:                  qgcPal.toolbarText
                    text:                   getBatteryVoltageText()
                    font.pointSize:         ScreenTools.mediumFontPointSize
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
                case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_EMERGENCY:
                case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_FAILED:
                case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_UNHEALTHY:
                    return 3
                case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_CRITICAL:
                    return 2
                case MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_LOW:
                    return 1
                default:
                    return 0
                }
            }

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

            SettingsGroupLayout {
                popoverStyle: true
                Layout.fillWidth:   true
                showDividers:       true
                visible:            control.totalPowerText() !== ""

                LabelledLabel {
                    label:      qsTr("Total draw")
                    labelText:  control.totalPowerText()
                }
            }

            Repeater {
                model: _batteries

                SettingsGroupLayout {
                    popoverStyle: true
                    Layout.fillWidth:   true
                    heading:            _batteryCount === 1 ? "" : qsTr("Battery %1").arg(index + 1)
                    contentSpacing:     0
                    showDividers:       true
    
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
                    property bool functionAvailable:         battery.function.rawValue !== MAVLinkEnums.MAV_BATTERY_FUNCTION_UNKNOWN
                    property bool showFunction:              functionAvailable && battery.function.rawValue != MAVLinkEnums.MAV_BATTERY_FUNCTION_ALL
                    property bool temperatureAvailable:      !isNaN(battery.temperature.rawValue)
                    property bool currentAvailable:          !isNaN(battery.current.rawValue)
                    property bool mahConsumedAvailable:      !isNaN(battery.mahConsumed.rawValue)
                    property bool timeRemainingAvailable:    !isNaN(battery.timeRemaining.rawValue)
                    property bool percentRemainingAvailable: !isNaN(battery.percentRemaining.rawValue)
                    property bool chargeStateAvailable:      battery.chargeState.rawValue !== MAVLinkEnums.MAV_BATTERY_CHARGE_STATE_UNDEFINED
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
                popoverStyle: true
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
                                    _batterySettings.setThreshold1(parseInt(text));
                                }
                            }
                        }

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
                                    _batterySettings.setThreshold2(parseInt(text));                                
                                }
                            }
                        }

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
                Layout.fillWidth:   true
                source:             _activeVehicle ? _activeVehicle.expandedToolbarIndicatorSource("Battery") : ""
            }

            SettingsGroupLayout {
                popoverStyle: true
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
