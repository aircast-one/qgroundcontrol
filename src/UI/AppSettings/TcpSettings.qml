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
import QGroundControl.ScreenTools
import QGroundControl.Palette

GridLayout {
    columns:        2
    rowSpacing:     _rowSpacing
    columnSpacing:  _colSpacing

    function saveSettings() {
        subEditConfig.host = hostField.text
        subEditConfig.port = parseInt(portField.text)
    }

    function validate() {
        const port = Number(portField.text)
        hostField.validationError = hostField.text.trim() === ""
        portField.validationError = !(/^\d+$/.test(portField.text) && port > 0 && port < 65536)
        return !hostField.validationError && !portField.validationError
    }

    function suggestedName() {
        return hostField.text.trim() === "" ? qsTr("TCP") : qsTr("TCP %1:%2").arg(hostField.text.trim()).arg(portField.text)
    }

    QGCLabel { text: qsTr("Server Address"); Layout.preferredWidth: _firstColumnWidth }
    QGCTextField {
        id:                     hostField
        Layout.preferredWidth:  _secondColumnWidth
        inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhNoPredictiveText
        text:                   subEditConfig.host
    }

    QGCLabel {
        Layout.columnSpan:  2
        Layout.leftMargin:  _firstColumnWidth + _colSpacing
        visible:        hostField.validationError
        color:          QGroundControl.globalPalette.colorRed
        font.pointSize: ScreenTools.smallFontPointSize
        text:           qsTr("Enter the drone's IP address")
    }

    QGCLabel { text: qsTr("Port"); Layout.preferredWidth: _firstColumnWidth }
    QGCTextField {
        id:                     portField
        Layout.preferredWidth:  _secondColumnWidth
        text:                   subEditConfig.port.toString()
        inputMethodHints:       Qt.ImhFormattedNumbersOnly
    }

    QGCLabel {
        Layout.columnSpan:  2
        Layout.leftMargin:  _firstColumnWidth + _colSpacing
        visible:        portField.validationError
        color:          QGroundControl.globalPalette.colorRed
        font.pointSize: ScreenTools.smallFontPointSize
        text:           qsTr("Enter a port between 1 and 65535")
    }
}
