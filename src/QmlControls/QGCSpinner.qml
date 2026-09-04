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
    id:     _root
    width:  ScreenTools.defaultFontPixelHeight * 1.1
    height: width

    property color color: QGroundControl.globalPalette.text

    readonly property real _stroke: Math.max(2, width * 0.13)

    onColorChanged: canvas.requestPaint()

    Canvas {
        id:           canvas
        anchors.fill: parent
        onPaint: {
            const ctx = getContext("2d")
            const centre = width / 2
            const radius = centre - _root._stroke / 2
            ctx.reset()
            ctx.lineWidth = _root._stroke
            ctx.lineCap   = "round"
            ctx.strokeStyle = Qt.alpha(_root.color, 0.25)
            ctx.beginPath()
            ctx.arc(centre, centre, radius, 0, Math.PI * 2)
            ctx.stroke()
            ctx.strokeStyle = _root.color
            ctx.beginPath()
            ctx.arc(centre, centre, radius, -Math.PI / 2, Math.PI * 0.15)
            ctx.stroke()
        }
    }

    RotationAnimator on rotation {
        running:    _root.visible
        from:       0
        to:         360
        duration:   900
        loops:      Animation.Infinite
    }
}
