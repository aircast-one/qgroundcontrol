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

Canvas {
    id: _root

    property real  progress:     -1
    property bool  done:         false
    property color contentColor: QGroundControl.globalPalette.text

    readonly property var _qgcPal: QGroundControl.globalPalette

    width:    ScreenTools.defaultFontPixelHeight * 1.1
    height:   width
    rotation: progress < 0 && !done ? angle : 0

    property real angle: 0

    NumberAnimation on angle {
        from:     0
        to:       360
        duration: 900
        loops:    Animation.Infinite
        running:  _root.visible && _root.progress < 0 && !_root.done
    }

    onPaint: {
        const ctx = getContext("2d")
        const r = width / 2 - 1.5
        ctx.reset()
        ctx.lineWidth = 2
        ctx.lineCap = "round"
        ctx.strokeStyle = Qt.alpha(_root.contentColor, 0.2)
        ctx.beginPath()
        ctx.arc(width / 2, height / 2, r, 0, Math.PI * 2)
        ctx.stroke()
        ctx.strokeStyle = _root.done ? _root._qgcPal.colorGreen : _root._qgcPal.colorBlue
        const sweep = _root.done ? 1 : _root.progress < 0 ? 0.25 : _root.progress
        ctx.beginPath()
        ctx.arc(width / 2, height / 2, r, -Math.PI / 2, -Math.PI / 2 + Math.PI * 2 * sweep)
        ctx.stroke()
        if (_root.done) {
            ctx.beginPath()
            ctx.moveTo(width * 0.32, height * 0.52)
            ctx.lineTo(width * 0.45, height * 0.66)
            ctx.lineTo(width * 0.70, height * 0.36)
            ctx.stroke()
        }
    }

    onProgressChanged:     requestPaint()
    onDoneChanged:         requestPaint()
    onContentColorChanged: requestPaint()
}
