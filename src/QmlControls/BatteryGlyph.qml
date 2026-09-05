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
import QGroundControl.Controls
import QGroundControl.ScreenTools

Item {
    id: _root

    property real fill:     0
    property bool charging: false
    property color color:   QGroundControl.globalPalette.toolbarText

    height: ScreenTools.defaultFontPixelHeight * 1.15
    width:  Math.round(height * 1.9)

    readonly property real _clamped: Math.max(0, Math.min(1, fill))

    Rectangle {
        id:             shell
        anchors.left:   parent.left
        width:          parent.width - nub.width
        height:         parent.height
        radius:         height * 0.3
        color:          "transparent"
        border.color:   _root.color

        border.width:   Math.max(1, Math.round(parent.height * 0.07))

        Rectangle {
            anchors.left:           parent.left
            anchors.leftMargin:     shell.border.width * 2
            anchors.verticalCenter: parent.verticalCenter
            height:                 parent.height - shell.border.width * 4
            width:                  (parent.width - shell.border.width * 4) * _root._clamped
            radius:                 height * 0.25
            color:                  _root.color

            Behavior on width { NumberAnimation { duration: 300; easing.type: Easing.OutCubic } }
        }
    }

    Rectangle {
        id:                     nub
        anchors.left:           shell.right
        anchors.verticalCenter: parent.verticalCenter
        width:                  parent.height * 0.14
        height:                 parent.height * 0.42
        radius:                 width / 2
        color:                  _root.color
    }

    QGCLabel {
        anchors.centerIn:   shell
        text:               "⚡"
        color:              QGroundControl.globalPalette.toolbarText
        font.pointSize:     ScreenTools.smallFontPointSize
        visible:            _root.charging
    }
}
