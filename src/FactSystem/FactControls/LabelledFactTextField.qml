/****************************************************************************
 *
 * (c) 2009-2022 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.FactSystem
import QGroundControl.FactControls

RowLayout {
    property string label:                   fact.shortDescription
    property alias  fact:                    _factTextField.fact
    property real   textFieldPreferredWidth: -1
    property alias  textFieldUnitsLabel:     _factTextField.unitsLabel
    property alias  textFieldShowUnits:      _factTextField.showUnits
    property alias  textFieldShowHelp:       _factTextField.showHelp
    property alias  textField:               _factTextField

    spacing:                ScreenTools.defaultFontPixelWidth * 2
    Layout.preferredHeight: ScreenTools.settingsRowHeight

    QGCLabel {
        Layout.fillWidth:   true
        elide:              Text.ElideRight
        Layout.alignment:   Qt.AlignVCenter
        text:               label
    }

    FactTextField {
        id:                     _factTextField
        Layout.preferredWidth:  textFieldPreferredWidth
        Layout.preferredHeight: ScreenTools.settingsRowHeight
        Layout.alignment:       Qt.AlignVCenter
        showFrame:              false
        horizontalAlignment:    TextInput.AlignRight
    }
}

