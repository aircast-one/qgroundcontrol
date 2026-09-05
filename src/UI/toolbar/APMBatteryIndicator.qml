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
import MAVLink

BatteryIndicator {
    waitForParameters: true

    expandedPageComponent: Component {
        ColumnLayout {
            id: expandedRoot

            FactPanelController { id: controller }

            // Guarded: asking for a parameter this firmware does not have pops a modal
            // "Parameters are missing from firmware" alert over the fly view, then draws the
            // control anyway with a meaningless value in it.
            readonly property bool _hasBattMonitor: controller.parameterExists(-1, "BATT_MONITOR")
            readonly property bool _hasBattFsLowAct: controller.parameterExists(-1, "BATT_FS_LOW_ACT")
            readonly property bool _hasBattLowVolt: controller.parameterExists(-1, "BATT_LOW_VOLT")
            readonly property bool _hasBattLowMah: controller.parameterExists(-1, "BATT_LOW_MAH")
            readonly property bool _hasBattFsCrtAct: controller.parameterExists(-1, "BATT_FS_CRT_ACT")
            readonly property bool _hasBattCrtVolt: controller.parameterExists(-1, "BATT_CRT_VOLT")
            readonly property bool _hasBattCrtMah: controller.parameterExists(-1, "BATT_CRT_MAH")

            property Fact batt1Monitor: expandedRoot._hasBattMonitor ? controller.getParameterFact(-1, "BATT_MONITOR") : null
            property string disabledString: qsTr("- disabled")

            SettingsGroupLayout {
                Layout.fillWidth:   true
                heading:            qsTr("Low Voltage Failsafe")
                visible:            batt1Monitor && batt1Monitor.rawValue !== 0

                LabelledFactComboBox {
                    label:              qsTr("Vehicle Action")
                    fact:               expandedRoot._hasBattFsLowAct ? controller.getParameterFact(-1, "BATT_FS_LOW_ACT") : null
                    visible:            expandedRoot._hasBattFsLowAct
                    indexModel:         false
                }

                FactSlider {
                    Layout.fillWidth:   true
                    label:              qsTr("Voltage Trigger") + (value == 0 ? disabledString : "")
                    fact:               expandedRoot._hasBattLowVolt ? controller.getParameterFact(-1, "BATT_LOW_VOLT") : null
                    visible:            expandedRoot._hasBattLowVolt
                    from:               0
                    to:                 100
                    majorTickStepSize:  5
                }

                FactSlider {
                    Layout.fillWidth:   true
                    label:              qsTr("mAh Trigger") + (value == 0 ? disabledString : "")
                    fact:               expandedRoot._hasBattLowMah ? controller.getParameterFact(-1, "BATT_LOW_MAH") : null
                    visible:            expandedRoot._hasBattLowMah
                    from:               0
                    to:                 30000
                    majorTickStepSize:  1000
                }
            }

            SettingsGroupLayout {
                Layout.fillWidth:   true
                heading:            qsTr("Critical Voltage Failsafe")
                visible:            batt1Monitor && batt1Monitor.rawValue !== 0

                LabelledFactComboBox {
                    label:              qsTr("Vehicle Action")
                    fact:               expandedRoot._hasBattFsCrtAct ? controller.getParameterFact(-1, "BATT_FS_CRT_ACT") : null
                    visible:            expandedRoot._hasBattFsCrtAct
                    indexModel:         false
                }

                FactSlider {
                    Layout.fillWidth:   true
                    label:              qsTr("Voltage Trigger") + (value == 0 ? disabledString : "")
                    fact:               expandedRoot._hasBattCrtVolt ? controller.getParameterFact(-1, "BATT_CRT_VOLT") : null
                    visible:            expandedRoot._hasBattCrtVolt
                    from:               0
                    to:                 100
                    majorTickStepSize:  5
                }

                FactSlider {
                    Layout.fillWidth:   true
                    label:              qsTr("mAh Trigger") + (value == 0 ? disabledString : "")
                    fact:               expandedRoot._hasBattCrtMah ? controller.getParameterFact(-1, "BATT_CRT_MAH") : null
                    visible:            expandedRoot._hasBattCrtMah
                    from:               0
                    to:                 30000
                    majorTickStepSize:  1000
                }
            }
        }
    }
}
