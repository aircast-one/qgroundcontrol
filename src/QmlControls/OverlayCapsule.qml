/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Effects

import QGroundControl
import QGroundControl.ScreenTools

Rectangle {
    id: _root

    property bool highlight: false
    property bool lifted:    false
    property bool frosted:   true

    readonly property var  _qgcPal:   QGroundControl.globalPalette
    readonly property Item _backdrop: typeof mainWindow === "undefined" ? null : mainWindow.frostedBackdrop
    readonly property bool _frosted:  frosted && _backdrop !== null

    readonly property rect _backdropRect: (_root.x, _root.y, _root.width, _root.height, _frosted
                                            ? _root.mapToItem(_backdrop, 0, 0, _root.width, _root.height)
                                            : Qt.rect(0, 0, 1, 1))

    height:         ScreenTools.defaultFontPixelHeight * 2.2
    radius:         height / 2
    color:          _frosted ? "transparent" : _qgcPal.overlayBackground
    border.width:   0
    layer.enabled:  true
    layer.effect:   OverlayShadowEffect { elevated: _root.lifted }

    ShaderEffectSource {
        id:            frost
        anchors.fill:  parent
        visible:       _root._frosted
        sourceItem:    _root._backdrop
        sourceRect:    _root._backdropRect
        layer.enabled: true
        layer.effect:  MultiEffect {
            maskEnabled:      true
            maskSource:       frostMask
            maskThresholdMin: 0.5
            maskSpreadAtMin:  1.0
        }
    }

    Item {
        id:            frostMask
        anchors.fill:  parent
        layer.enabled: true
        visible:       false

        Rectangle {
            anchors.fill: parent
            radius:       _root.radius
            color:        "black"
        }
    }

    Rectangle {
        anchors.fill: parent
        radius:       _root.radius
        color:        _root._frosted ? _root._qgcPal.overlayGlass : "transparent"
        border.width: 1
        border.color: _root.highlight ? Qt.alpha(_root._qgcPal.text, 0.6)
                                      : _root._qgcPal.overlayBorder
    }
}
