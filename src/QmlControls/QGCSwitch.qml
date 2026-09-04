/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls

import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.ScreenTools

Switch {
    id: control

    QGCPalette {
        id:                 qgcPal
        colorGroupEnabled:  true
    }

    contentItem: QGCLabel {
        text:               control.text
        verticalAlignment:  Text.AlignVCenter
        rightPadding:       control.indicator.width + control.spacing
    }

    indicator: Rectangle {
        implicitWidth:  Math.round(implicitHeight * 1.65)
        implicitHeight: Math.round(ScreenTools.defaultFontPixelHeight * 1.3)
        x:              control.width - width - control.rightPadding
        y:              parent.height / 2 - height / 2
        radius:         height / 2
        color:          control.checked ? qgcPal.colorGreen : qgcPal.windowShadeLight

        Behavior on color { ColorAnimation { duration: 150 } }

        Rectangle {
            id:                     knob
            anchors.verticalCenter: parent.verticalCenter
            x:                      control.checked ? parent.width - width - 2 : 2
            width:                  parent.height - 4
            height:                 width
            radius:                 height / 2
            color:                  "#ffffff"
            border.color:           "#1f000000"
            border.width:           1

            Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
        }
    }
}
