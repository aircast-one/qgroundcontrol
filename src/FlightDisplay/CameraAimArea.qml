/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

MouseArea {
    id:             _root
    enabled:        false
    visible:        enabled
    preventStealing: false
    cursorShape:    pressed ? Qt.ClosedHandCursor : Qt.OpenHandCursor

    property real sensitivity: 1 / (Math.max(width, 1) / 2)

    signal aimed(real panFraction, real tiltFraction)
    signal zoomed(real zoomFraction)

    readonly property real _minPinchScale: 0.1
    readonly property real _maxPinchScale: 10

    property real _lastX: 0
    property real _lastY: 0

    onPressed: (mouse) => {
        _lastX = mouse.x
        _lastY = mouse.y
    }

    onPositionChanged: (mouse) => {
        if (!pressed) {
            return
        }
        _root.aimed((mouse.x - _lastX) * sensitivity, -(mouse.y - _lastY) * sensitivity)
        _lastX = mouse.x
        _lastY = mouse.y
    }

    PinchHandler {
        target:         null
        minimumScale:   _minPinchScale
        maximumScale:   _maxPinchScale

        property real _lastScale: 1

        onActiveChanged: {
            if (active) {
                _lastScale = activeScale
            }
        }

        onActiveScaleChanged: {
            if (!active) {
                return
            }
            _root.zoomed(activeScale - _lastScale)
            _lastScale = activeScale
        }
    }
}
