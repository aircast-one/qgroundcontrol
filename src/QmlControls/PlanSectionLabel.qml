/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

QGCLabel {
    width:          parent ? parent.width : implicitWidth
    leftPadding:    ScreenTools.defaultFontPixelHeight / 2
    topPadding:     ScreenTools.defaultFontPixelHeight * 0.3
    font.pointSize: ScreenTools.smallFontPointSize
    font.bold:      true
    color:          Qt.alpha(QGroundControl.globalPalette.text, 0.5)
}
