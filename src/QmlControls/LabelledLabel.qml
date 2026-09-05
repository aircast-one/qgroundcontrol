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

RowLayout {
    property alias label:                   _labelLabel.text
    property alias labelText:              _label.text
    property alias labelTextColor:          _label.color
    property real  labelPreferredWidth:    -1
    property real  fontPointSize:          ScreenTools.defaultFontPointSize

    spacing:                ScreenTools.defaultFontPixelWidth * 2
    Layout.preferredHeight: ScreenTools.settingsRowHeight

    QGCLabel { 
        id:                 _labelLabel
        Layout.fillWidth:   true 
        font.pointSize:     fontPointSize
    }

    QGCLabel {
        id:                     _label
        Layout.preferredWidth:  labelPreferredWidth
        font.pointSize:         fontPointSize
    }
}

