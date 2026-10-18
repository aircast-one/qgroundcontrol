import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls

SettingsGroupLayout {
    heading:            qsTr("Radio on USB (EdgeTX script)")
    headingDescription: qsTr("A radio running the Aircast USB script flies the active vehicle with its sticks and switches.")
    visible:            _usbRadio !== null

    readonly property var _usbRadio:            QGroundControl.usbRadio
    readonly property var _autoConnectSettings: QGroundControl.settingsManager.autoConnectSettings
    property var          _ports:               []

    function _refresh() {
        _ports = _usbRadio.ports()
        portCombo.model = _ports.map(row => row.label)
        portCombo.currentIndex = Math.max(0, _ports.findIndex(row => row.port === _autoConnectSettings.usbRadioPort.rawValue))
    }

    LabelledComboBox {
        id:                 portCombo
        Layout.fillWidth:   true
        label:              qsTr("Device")
        onActivated:        (index) => _autoConnectSettings.usbRadioPort.rawValue = _ports[index].port
        Component.onCompleted: _refresh()
    }

    FactCheckBoxSlider {
        Layout.fillWidth:   true
        text:               fact.shortDescription
        fact:               _autoConnectSettings.usbRadioStandby
        visible:            _autoConnectSettings.usbRadioPort.rawValue !== ""
    }

    QGCLabel {
        Layout.fillWidth:   true
        text:               _usbRadio ? _usbRadio.status : ""
        color:              _usbRadio && _usbRadio.receiving ? qgcPal.colorGreen : qgcPal.text
        visible:            _autoConnectSettings.usbRadioPort.rawValue !== ""
        wrapMode:           Text.WordWrap
    }

    Timer {
        interval:       2000
        repeat:         true
        running:        _usbRadio !== null
        onTriggered:    if (!portCombo.comboBox.popup.visible) _refresh()
    }

    QGCPalette { id: qgcPal }
}
