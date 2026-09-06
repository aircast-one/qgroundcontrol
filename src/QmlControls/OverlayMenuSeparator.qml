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
import QGroundControl.ScreenTools

// A hairline between groups of menu rows. Sets destructive or state-changing entries apart from
// the routine ones above them, so the last row is reached deliberately rather than by momentum.
Rectangle {
    Layout.fillWidth:       true
    Layout.preferredHeight: 1
    Layout.topMargin:       ScreenTools.defaultFontPixelHeight * 0.3
    Layout.bottomMargin:    ScreenTools.defaultFontPixelHeight * 0.3
    Layout.leftMargin:      ScreenTools.defaultFontPixelWidth * 1.5
    Layout.rightMargin:     ScreenTools.defaultFontPixelWidth * 1.5
    color:                  Qt.alpha(QGroundControl.globalPalette.text, 0.15)
}
