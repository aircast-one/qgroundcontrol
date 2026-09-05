/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.Controls
import QGroundControl.ScreenTools

// Small outlined caption drawn under an overlay control (camera chrome, RC controls).
QGCLabel {
    anchors.horizontalCenter:   parent.horizontalCenter
    font.pointSize:             ScreenTools.smallFontPointSize
    style:                      Text.Outline
    styleColor:                 "black"
}
