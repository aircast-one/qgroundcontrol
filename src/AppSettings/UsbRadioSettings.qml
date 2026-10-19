import QtQuick
import QtQuick.Layouts
import QtCore

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls

SettingsGroupLayout {
    heading:            qsTr("Radio on USB (EdgeTX script)")
    headingDescription: qsTr("A radio running the Aircast USB script flies the active vehicle with its sticks and switches. The sticks travel over whatever link this app uses to reach the aircraft: connect the aircraft over Starlink or 4G to fly over it.")
    visible:            _usbRadio !== null

    readonly property var _usbRadio:            QGroundControl.usbRadio
    readonly property var _autoConnectSettings: QGroundControl.settingsManager.autoConnectSettings
    property var          _ports:               []
    property string       _saved:               ""
    property string       _card:                ""
    readonly property string _guide:            "https://aircast.one/docs/software/usb-radio"

    function _refresh() {
        _ports = _usbRadio.ports()
        _card = _usbRadio.scriptFolder()
        portCombo.model = _ports.map(row => row.label)
        portCombo.currentIndex = Math.max(0, _ports.findIndex(row => row.port === _autoConnectSettings.usbRadioPort.rawValue))
    }

    QGCLabel {
        Layout.fillWidth:   true
        wrapMode:           Text.WordWrap
        text:               qsTr("1. The radio needs EdgeTX 2.10 or newer: on 2.9 it never hears this app.\n" +
                                 "2. Save the radio script below into SCRIPTS/TOOLS on the radio's SD card.\n" +
                                 "3. On the radio set SYS → Hardware → USB-VCP to LUA.\n" +
                                 "4. Plug the radio in, choose USB Serial (VCP), and open SYS → Tools → Aircast USB radio.\n" +
                                 "5. The app asks \"Fly with it?\" once the radio streams: press Use, or pick the radio as Device here. The radio's screen then says \"App flying vehicle N\".")
    }

    RowLayout {
        spacing: ScreenTools.defaultFontPixelWidth

        QGCButton {
            text:       qsTr("Save radio script…")
            onClicked: {
                _saved = ""
                _card = _usbRadio.scriptFolder()
                scriptFolderDialog.folder = _card !== "" ? _card : QGCFileDialogController.urlToLocalFile(StandardPaths.writableLocation(StandardPaths.DocumentsLocation))
                scriptFolderDialog.title = _card !== "" ? qsTr("Save aircast-usb.lua to the radio's SD card") : qsTr("Save aircast-usb.lua")
                scriptFolderDialog.openForLoad()
            }
        }

        QGCLabel {
            text:               "<a href=\"" + _guide + "\">" + qsTr("Setup guide") + "</a>"
            textFormat:         Text.RichText
            onLinkActivated:    (link) => Qt.openUrlExternally(link)
        }
    }

    QGCLabel {
        Layout.fillWidth:   true
        wrapMode:           Text.WordWrap
        visible:            _card !== "" && _saved === ""
        text:               qsTr("Radio SD card found: the script saves to %1").arg(_card)
    }

    QGCLabel {
        Layout.fillWidth:   true
        wrapMode:           Text.WordWrap
        visible:            _saved !== ""
        text:               _saved
    }

    QGCFileDialog {
        id:             scriptFolderDialog
        selectFolder:   true
        onAcceptedForLoad: (folder) => {
            const error = _usbRadio.saveScript(folder)
            _saved = error !== "" ? error : qsTr("Saved aircast-usb.lua to %1. Eject the card before unplugging the radio.").arg(folder)
        }
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
