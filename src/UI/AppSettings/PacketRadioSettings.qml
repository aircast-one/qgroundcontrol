import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Controls
import QGroundControl.ScreenTools

SettingsPage {
    property var    _packetRadioSettings:   QGroundControl.settingsManager.packetRadioSettings
    property var    _radio:                 QGroundControl.packetRadioManager
    property Fact   _enabled:               _packetRadioSettings.enabled
    property Fact   _deviceName:            _packetRadioSettings.deviceName

    Component.onCompleted: {
        if (_radio) {
            _radio.refreshAdapters()
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               fact.shortDescription
            fact:               _enabled
            visible:            fact.visible
        }

        LabelledLabel {
            Layout.fillWidth:   true
            label:              qsTr("Status")
            labelText:          _radio ? _radio.statusText : qsTr("Unavailable")
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Link quality")
        visible:            _radio && _radio.linkActive

        LabelledLabel {
            Layout.fillWidth:   true
            label:              qsTr("Signal per antenna")
            labelText:          _radio && _radio.haveSignal ? _radio.antennaRssi.join("  |  ") + qsTr(" dBm") : qsTr("waiting")
        }

        LabelledLabel {
            Layout.fillWidth:   true
            label:              qsTr("Noise margin per antenna")
            labelText:          _radio && _radio.haveSignal ? _radio.antennaSnr.join("  |  ") + qsTr(" dB") : qsTr("waiting")
        }

        LabelledLabel {
            Layout.fillWidth:   true
            label:              qsTr("Link score (1000-2000)")
            labelText:          _radio && _radio.haveSignal ? _radio.linkScore.toString() : qsTr("waiting")
        }

        LabelledLabel {
            Layout.fillWidth:   true
            label:              qsTr("Packets lost (last second)")
            labelText:          _radio ? _radio.packetLoss.toString() : ""
        }

        LabelledLabel {
            Layout.fillWidth:   true
            label:              qsTr("Video packets")
            labelText:          _radio ? _radio.videoPackets.toString() : ""
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Radio")
        enabled:            _enabled.rawValue

        LabelledComboBox {
            Layout.fillWidth:   true
            label:              qsTr("Wi-Fi adapter")
            model:              [ qsTr("Automatic") ].concat(_radio ? _radio.adapters : [])
            currentIndex:       {
                if (!_radio || _deviceName.rawValue === "") {
                    return 0
                }
                const found = _radio.adapters.indexOf(_deviceName.rawValue)
                return found < 0 ? 0 : found + 1
            }
            onActivated: (index) => {
                _deviceName.rawValue = index === 0 ? "" : _radio.adapters[index - 1]
            }
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              fact.shortDescription
            fact:               _packetRadioSettings.channel
            visible:            fact.visible
        }

        LabelledFactComboBox {
            Layout.fillWidth:   true
            label:              fact.shortDescription
            fact:               _packetRadioSettings.channelWidth
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              fact.shortDescription
            fact:               _packetRadioSettings.keyFile
            visible:            fact.visible
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Adaptive link")
        enabled:            _enabled.rawValue

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               fact.shortDescription
            fact:               _packetRadioSettings.alinkEnabled
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              fact.shortDescription
            fact:               _packetRadioSettings.alinkTxPower
            visible:            fact.visible
            enabled:            _packetRadioSettings.alinkEnabled.rawValue
        }
    }
}
