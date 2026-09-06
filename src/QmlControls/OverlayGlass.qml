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

Item {
    id: _root

    property real  radius:    height / 2
    property bool  highlight: false
    property bool  frosted:   true
    property color tint:      defaultTint
    readonly property color defaultTint:   _panel ? Qt.alpha(_qgcPal.window, 0.94) : _qgcPal.overlayGlass
    readonly property color contentColor:  _panel ? _qgcPal.text   : _qgcPal.overlayInk
    readonly property color invertedColor: _panel ? _qgcPal.window : _qgcPal.overlayInkInverse

    enum Material { Control, Panel }

    property int   material:  OverlayGlass.Control
    property real  blurRadius: ScreenTools.defaultFontPixelHeight * (_panel ? 1.0 : 0.7) * frost
    property Item  backdrop:  OverlayBackdrop.forItem(_root)
    property real  frost: QGroundControl.settingsManager.appSettings.overlayGlassFrost.rawValue / 100

    readonly property bool _panel:   material === OverlayGlass.Panel
    readonly property real _minTint: (_panel ? 0.48 : 0.35) * frost
    readonly property real _maxTint: (_panel ? 0.66 : 0.85) * frost

    readonly property bool active: frosted && _backdrop !== null

    readonly property var  _qgcPal:   QGroundControl.globalPalette
    readonly property Item _backdrop: backdrop

    function _transformKey(item) {
        let key = 0
        for (let it = item; it; it = it.parent) {
            key += it.x + it.y + it.scale + it.rotation
        }
        return key
    }

    readonly property real _transforms: _transformKey(_root) + (_backdrop ? _transformKey(_backdrop) : 0)

    readonly property rect _backdropRect: {
        if (!active) {
            return Qt.rect(0, 0, 1, 1)
        }
        void _transforms
        const topLeft = _root.mapToItem(_backdrop, 0, 0)
        const bottomRight = _root.mapToItem(_backdrop, _root.width, _root.height)
        return Qt.rect(topLeft.x, topLeft.y, bottomRight.x - topLeft.x, bottomRight.y - topLeft.y)
    }

    Rectangle {
        anchors.fill: parent
        visible:      !_root.active
        radius:       _root.radius
        color:        _root._panel ? _root._qgcPal.window : _root._qgcPal.overlayBackground
        border.width: 1
        border.color: _root.highlight ? Qt.alpha(_root.contentColor, 0.6) : _root._qgcPal.overlayBorder
    }

    ShaderEffectSource {
        id:           backdropCrop
        anchors.fill: parent
        visible:      false
        live:         false
        sourceItem:   _root._backdrop
        sourceRect:   _root._backdropRect

        Connections {
            target:               OverlayBackdrop
            function onRefreshed() { backdropCrop.scheduleUpdate() }
        }
    }

    on_BackdropRectChanged: OverlayBackdrop.refresh()

    onActiveChanged: if (active) OverlayBackdrop.refresh()

    ShaderEffect {
        anchors.fill:   parent
        visible:        _root.active
        fragmentShader: "qrc:/shaders/LiquidGlass.frag.qsb"

        readonly property variant source:     backdropCrop
        readonly property color   tintColor:  _root.tint
        readonly property vector2d size:      Qt.vector2d(width, height)
        readonly property real    radius:     _root.radius
        readonly property real    minTint:    _root._minTint
        readonly property real    maxTint:    _root._maxTint
        readonly property real    rimOpacity: _root.highlight ? 0.55 : 0.3
        readonly property real    refraction: Math.min(_root.radius, 12) * 0.9
        readonly property real    blurRadius: _root.blurRadius
    }
}
