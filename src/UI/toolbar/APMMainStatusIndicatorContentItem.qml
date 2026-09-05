/****************************************************************************
 *
 * (c) 2009-2022 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.FactSystem
import QGroundControl.FactControls

ColumnLayout {
    id:      control
    spacing: ScreenTools.defaultFontPixelHeight / 2

    FactPanelController { id: controller }

    // Not every ArduPilot build and version carries these. Asking for one that is absent pops a
    // modal "Parameters are missing from firmware" alert over the fly view - and then draws the
    // control anyway, with a meaningless 0.000 in it. Ask whether it exists first and simply do
    // not offer what this vehicle does not have.
    readonly property bool _hasGcsEnable:  controller.parameterExists(-1, "FS_GCS_ENABLE")
    readonly property bool _hasGcsTimeout: controller.parameterExists(-1, "FS_GCS_TIMEOUT")
    readonly property bool _hasOptions:    controller.parameterExists(-1, "FS_OPTIONS")

    SettingsGroupLayout {
        heading:            qsTr("Ground Control Comm Loss Failsafe")
        Layout.fillWidth:   true
        popoverStyle:       true
        visible:            control._hasGcsEnable || control._hasGcsTimeout

        LabelledFactComboBox {
            label:      qsTr("Vehicle Action")
            visible:    control._hasGcsEnable
            fact:       control._hasGcsEnable ? controller.getParameterFact(-1, "FS_GCS_ENABLE") : null
            indexModel: false
        }

        FactSlider {
            Layout.fillWidth:       true
            Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 20
            label:                  qsTr("Loss Timeout")
            visible:                control._hasGcsTimeout
            fact:                   control._hasGcsTimeout ? controller.getParameterFact(-1, "FS_GCS_TIMEOUT") : null
            majorTickStepSize:      5
        }
    }

    SettingsGroupLayout {
        heading:            qsTr("Failsafe Options")
        Layout.fillWidth:   true
        popoverStyle:       true
        visible:            control._hasOptions

        Repeater {
            id:     repeater
            model:  fact ? fact.bitmaskStrings : []

            property Fact fact: control._hasOptions ? controller.getParameterFact(-1, "FS_OPTIONS") : null

            QGCCheckBoxSlider {
                Layout.fillWidth: true
                text:               modelData
                checked:            fact.value & fact.bitmaskValues[index]

                property Fact fact: repeater.fact

                onClicked: {
                    var i
                    var otherCheckbox
                    if (checked) {
                        fact.value |= fact.bitmaskValues[index]
                    } else {
                        fact.value &= ~fact.bitmaskValues[index]
                    }
                }
            }
        }
    }
}
