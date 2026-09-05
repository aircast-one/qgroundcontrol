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
import QtQuick.Layouts

import QGroundControl.Palette
import QGroundControl.ScreenTools

AbstractButton   {
    id:             control
    checkable:      true
    padding:        0

    QGCPalette { id: qgcPal; colorGroupEnabled: control.enabled }

    contentItem: Item {
        implicitWidth:  (label.visible ? label.contentWidth + ScreenTools.defaultFontPixelWidth : 0) + indicator.width
        implicitHeight: ScreenTools.settingsRowHeight

        QGCLabel {
            id:                     label
            anchors.left:           parent.left
            anchors.verticalCenter: parent.verticalCenter
            text:                   visible ? control.text : "X"
            visible:                control.text !== ""
        }

        Rectangle {
            id:                     indicator
            anchors.right:          parent.right
            anchors.verticalCenter: parent.verticalCenter
            height:                 Math.round(ScreenTools.defaultFontPixelHeight * 1.3)
            width:                  Math.round(height * 1.65)
            radius:                 height / 2
            opacity:                control.enabled ? 1 : 0.4
            color:                  control.checked ? qgcPal.colorGreen : qgcPal.windowShadeLight

            Behavior on color { ColorAnimation { duration: 150 } }

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                x:                      control.checked ? indicator.width - width - 2 : 2
                height:                 parent.height - 4
                width:                  height
                radius:                 height / 2
                color:                  "#ffffff"
                border.color:           "#1f000000"
                border.width:           1

                Behavior on x { NumberAnimation { duration: 150; easing.type: Easing.OutCubic } }
            }
        }
    }
}
