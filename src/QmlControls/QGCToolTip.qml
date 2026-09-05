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

import QGroundControl
import QGroundControl.ScreenTools

// Themed replacement for the Basic-style white ToolTip: same overlay capsule as the rest of
// the on-picture chrome.
ToolTip {
    id:             control
    delay:          600
    leftPadding:    ScreenTools.defaultFontPixelWidth * 1.5
    rightPadding:   ScreenTools.defaultFontPixelWidth * 1.5
    topPadding:     ScreenTools.defaultFontPixelHeight / 3
    bottomPadding:  ScreenTools.defaultFontPixelHeight / 3

    readonly property var _qgcPal: QGroundControl.globalPalette

    contentItem: Text {
        text:           control.text
        font.pointSize: ScreenTools.smallFontPointSize
        font.family:    ScreenTools.normalFontFamily
        color:          control._qgcPal.text
        wrapMode:       Text.WordWrap
    }

    background: Rectangle {
        layer.enabled: true
        layer.effect:  OverlayShadowEffect { }
        color:          "transparent"
        radius:         height / 2

        OverlayGlass {
            anchors.fill: parent
            radius:       parent.radius
        }
    }
}
