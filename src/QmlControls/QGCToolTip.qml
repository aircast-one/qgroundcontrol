import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls

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
        color:          tipGlass.contentColor
        wrapMode:       Text.WordWrap
    }

    background: Rectangle {
        layer.enabled: true
        layer.effect:  OverlayShadowEffect { }
        color:          "transparent"
        radius:         height / 2

        OverlaySurface {
            id:           tipGlass
            anchors.fill: parent
            radius:       parent.radius
        }
    }
}
