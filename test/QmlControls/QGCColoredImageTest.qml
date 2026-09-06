/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.Controls

Item {
    width:  100
    height: 100

    readonly property var untinted: untintedImage
    readonly property var opaque:   opaqueImage
    readonly property var faded:    fadedImage

    QGCColoredImage {
        id:     untintedImage
        width:  20
        height: 20
        source: "/InstrumentValueIcons/drone.svg"
        color:  "transparent"
    }

    QGCColoredImage {
        id:     opaqueImage
        width:  20
        height: 20
        source: "/InstrumentValueIcons/drone.svg"
        color:  "red"
    }

    QGCColoredImage {
        id:     fadedImage
        width:  20
        height: 20
        source: "/InstrumentValueIcons/drone.svg"
        color:  Qt.rgba(1, 0, 0, 0.35)
    }
}
