/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.Palette

// Battery of the device QGC itself runs on (phone/tablet/laptop)
Item {
    id:             control
    anchors.top:    parent.top
    anchors.bottom: parent.bottom
    width:          gcsBatteryRow.width

    property bool showIndicator: _battery.available

    readonly property var  _battery: QGroundControl.gcsBattery
    readonly property color _color:  _battery.charging      ? qgcPal.colorGreen :
                                     _battery.level <= 10   ? qgcPal.colorRed :
                                     _battery.level <= 25   ? qgcPal.colorYellow :
                                                              qgcPal.toolbarText

    Row {
        id:             gcsBatteryRow
        anchors.top:    parent.top
        anchors.bottom: parent.bottom
        spacing:        ScreenTools.defaultFontPixelWidth * 0.6

        Item {
            id:                     glyph
            anchors.verticalCenter: parent.verticalCenter
            height:                 ScreenTools.defaultFontPixelHeight * 1.15
            width:                  height * 2

            Rectangle {
                id:             shell
                anchors.left:   parent.left
                width:          parent.width - nub.width
                height:         parent.height
                radius:         height * 0.3
                color:          "transparent"
                border.color:   control._color
                border.width:   Math.max(1, parent.height * 0.09)

                Rectangle {
                    anchors.left:           parent.left
                    anchors.leftMargin:     shell.border.width * 2
                    anchors.verticalCenter: parent.verticalCenter
                    height:                 parent.height - shell.border.width * 4
                    width:                  (parent.width - shell.border.width * 4) * Math.max(0, Math.min(1, control._battery.level / 100))
                    radius:                 height * 0.25
                    color:                  control._color
                }
            }

            Rectangle {
                id:                     nub
                anchors.left:           shell.right
                anchors.verticalCenter: parent.verticalCenter
                width:                  parent.height * 0.14
                height:                 parent.height * 0.42
                radius:                 width / 2
                color:                  control._color
            }

            QGCLabel {
                anchors.centerIn:   shell
                text:               "⚡"
                color:              qgcPal.toolbarText
                font.pointSize:     ScreenTools.smallFontPointSize
                visible:            control._battery.charging
            }
        }

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            font.pointSize:         ScreenTools.mediumFontPointSize
            color:                  qgcPal.toolbarText
            text:                   control._battery.level + "%"
        }
    }

    QGCToolTip {
        visible: mouseArea.containsMouse
        text:    control._battery.charging ? qsTr("Ground station battery — charging")
                                           : qsTr("Ground station battery")
    }

    QGCMouseArea {
        id:             mouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
    }
}
