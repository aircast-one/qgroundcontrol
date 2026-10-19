import QtQuick

import QGroundControl
import QGroundControl.Controls

Rectangle {
    id: _root

    property bool  highlight: false
    property bool  lifted:    false
    property color accent:    "transparent"

    readonly property color contentColor:  glass.contentColor
    readonly property color invertedColor: glass.invertedColor

    readonly property var _qgcPal: QGroundControl.globalPalette

    height:         ScreenTools.defaultFontPixelHeight * 2.2
    radius:         height / 2
    color:          "transparent"
    layer.enabled:  true
    layer.effect:   OverlayShadowEffect { elevated: _root.lifted }

    OverlaySurface {
        id:           glass
        anchors.fill: parent
        radius:       _root.radius
        highlight:    _root.highlight
        tint:         _root.accent.a > 0 ? _root.accent : glass.defaultTint
    }
}
