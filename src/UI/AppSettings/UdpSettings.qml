/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
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
import QGroundControl.FactControls
import QGroundControl.ScreenTools
import QGroundControl.Palette

ColumnLayout {
    spacing: _rowSpacing

    function saveSettings() {
        subEditConfig.localPort = _effectivePort()
        if (hostField.text.trim() !== "") {
            subEditConfig.addHost(hostField.text.trim())
        }
    }

    function suggestedName() {
        return qsTr("UDP %1").arg(_effectivePort())
    }

    function validate() {
        portField.validationError = portField.text.trim() !== "" && !_validPort()
        return !portField.validationError
    }

    function _validPort() {
        const port = Number(portField.text)
        return /^\d+$/.test(portField.text.trim()) && port > 0 && port < 65536
    }

    function _effectivePort() {
        return _validPort() ? Number(portField.text) : _defaultPort
    }

    readonly property int _defaultPort: QGroundControl.settingsManager.autoConnectSettings.udpListenPort.rawValue

    RowLayout {
        spacing: _colSpacing

        QGCLabel { text: qsTr("Port"); Layout.preferredWidth: _firstColumnWidth }
        QGCTextField {
            id:                     portField
            objectName:             "udpPortField"
            text:                   subEditConfig.localPort.toString()
            focus:                  true
            Layout.preferredWidth:  _secondColumnWidth
            inputMethodHints:       Qt.ImhFormattedNumbersOnly
        }
    }

    QGCLabel {
        Layout.leftMargin:  _firstColumnWidth + _colSpacing
        visible:            portField.validationError
        color:              QGroundControl.globalPalette.colorRed
        font.pointSize:     ScreenTools.smallFontPointSize
        text:               qsTr("Enter a port between 1 and 65535, or leave it blank for %1").arg(_defaultPort)
    }

    RowLayout {
        Layout.fillWidth:   true
        spacing:            _colSpacing

        ColumnLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelHeight * 0.1

            QGCLabel { text: qsTr("Auto Connect to UDP devices") }

            QGCLabel {
                Layout.fillWidth:   true
                wrapMode:           Text.WordWrap
                font.pointSize:     ScreenTools.smallFontPointSize
                color:              QGroundControl.globalPalette.colorGrey
                text:               qsTr("Turn this off for best performance with this link.")
            }
        }

        FactCheckBoxSlider {
            fact: QGroundControl.settingsManager.autoConnectSettings.autoConnectUDP
        }
    }

    QGCLabel { text: qsTr("Server Addresses (optional)") }

    Repeater {
        model: subEditConfig.hostList

        delegate: RowLayout {
            spacing: _colSpacing

            QGCLabel {
                Layout.preferredWidth:  _secondColumnWidth
                text:                   modelData
            }

            QGCButton {
                text:       qsTr("Remove")
                onClicked:  subEditConfig.removeHost(modelData)
            }
        }
    }

    RowLayout {
        spacing: _colSpacing

        QGCTextField {
            id:                     hostField
            objectName:             "udpHostField"
            Layout.preferredWidth:  _secondColumnWidth
            inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhNoPredictiveText
            placeholderText:        qsTr("Example: 127.0.0.1:14550")
        }
        QGCButton {
            text:       qsTr("Add Server")
            enabled:    hostField.text !== ""
            onClicked: {
                subEditConfig.addHost(hostField.text)
                hostField.text = ""
            }
        }
    }
}
