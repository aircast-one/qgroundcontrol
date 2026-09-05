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
import QGroundControl.ScreenTools

Rectangle {
    id: _root

    property bool  highlight: false
    property bool  lifted:    false
    property bool  frosted:   true
    property color accent:    "transparent"
    property alias backdrop:      glass.backdrop
    property alias lightMaterial: glass.lightMaterial

    readonly property color contentColor: glass.contentColor

    readonly property var _qgcPal: QGroundControl.globalPalette

    height:         ScreenTools.defaultFontPixelHeight * 2.2
    radius:         height / 2
    color:          "transparent"
    layer.enabled:  true
    layer.effect:   OverlayShadowEffect { elevated: _root.lifted }

    OverlayGlass {
        id:           glass
        anchors.fill: parent
        radius:       _root.radius
        highlight:    _root.highlight
        frosted:      _root.frosted
        // An accent tints the glass rather than covering it. Painting the accent on top threw
        // away the refraction and the rim and left a flat slab sitting on a glass panel.
        tint:         _root.accent.a > 0 ? _root.accent : _qgcPal.overlayGlass
    }
}
