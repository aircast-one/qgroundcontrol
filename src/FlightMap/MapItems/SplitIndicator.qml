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
import QGroundControl.ScreenTools

Rectangle {
    id: _root

    signal clicked

    width:         ScreenTools.defaultFontPixelHeight * 1.0
    height:        width
    radius:        width / 2
    color:         Qt.rgba(1, 1, 1, splitMouseArea.containsMouse ? 1 : 0.7)
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { }

    Behavior on color { ColorAnimation { duration: 120 } }

    Rectangle {
        anchors.centerIn: parent
        width:            parent.width * 0.5
        height:           2
        radius:           1
        color:            Qt.rgba(0, 0, 0, 0.7)
    }

    Rectangle {
        anchors.centerIn: parent
        width:            2
        height:           parent.width * 0.5
        radius:           1
        color:            Qt.rgba(0, 0, 0, 0.7)
    }

    QGCMouseArea {
        id:           splitMouseArea
        fillItem:     parent
        hoverEnabled: !ScreenTools.isMobile
        onClicked:    _root.clicked()
    }
}
