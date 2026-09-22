import QtQuick

import QGroundControl
import QGroundControl.Controls

Canvas {
    id:     control
    width:  ScreenTools.defaultFontPixelWidth * 36
    height: ScreenTools.defaultFontPixelHeight * 3

    property var    values:     []
    property real   maximum:    0
    property color  lineColor:  qgcPal.colorGreen

    QGCPalette { id: qgcPal }

    onValuesChanged:    requestPaint()
    onMaximumChanged:   requestPaint()

    onPaint: {
        var ctx = getContext("2d")
        ctx.clearRect(0, 0, width, height)
        var n = values.length
        if (n < 2) return
        var peak = maximum
        if (peak <= 0) {
            for (var k = 0; k < n; k++) peak = Math.max(peak, values[k])
        }
        if (peak <= 0) peak = 1
        var step = width / (n - 1)
        ctx.lineWidth = 1.5
        ctx.strokeStyle = lineColor
        var open = false
        for (var i = 0; i < n; i++) {
            var v = values[i]
            if (v < 0) {
                if (open) ctx.stroke()
                open = false
                continue
            }
            var x = i * step
            var y = height - Math.min(v, peak) / peak * (height - 1) - 0.5
            if (!open) {
                ctx.beginPath()
                ctx.moveTo(x, y)
                open = true
            } else {
                ctx.lineTo(x, y)
            }
        }
        if (open) ctx.stroke()
    }
}
