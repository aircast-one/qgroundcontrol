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

    property bool grip: false

    width:         ScreenTools.defaultFontPixelHeight * 1.2
    height:        width
    radius:        width / 2
    color:         "white"
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { }

    Grid {
        anchors.centerIn: parent
        visible:          _root.grip
        columns:          2
        spacing:          _root.width * 0.12

        Repeater {
            model: 4

            Rectangle {
                width:  _root.width * 0.15
                height: width
                radius: width / 2
                color:  Qt.rgba(0, 0, 0, 0.55)
            }
        }
    }
}
