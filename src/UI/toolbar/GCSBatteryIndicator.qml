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
    // Neutral while it is fine; a colour only when it wants attention. Matches the vehicle
    // packs exactly, so the two can be compared at a glance.
    readonly property color _color:  _battery.charging    ? qgcPal.colorGreen :
                                     _battery.level <= 10 ? qgcPal.colorRed :
                                     _battery.level <= 25 ? qgcPal.colorYellow :
                                                            qgcPal.toolbarText

    Row {
        id:             gcsBatteryRow
        anchors.top:    parent.top
        anchors.bottom: parent.bottom
        spacing:        ScreenTools.defaultFontPixelWidth * 0.6

        GcsBatteryGlyph {
            anchors.verticalCenter: parent.verticalCenter
            fill:                   control._battery.level / 100
            charging:               control._battery.charging
            color:                  control._color
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

    // The only indicator in the row that could not be opened. A hover tooltip is nothing on a
    // touchscreen, which is where this runs.
    QGCMouseArea {
        id:             mouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
        onClicked:      mainWindow.showIndicatorDrawer(gcsBatteryPage, control)
    }

    Component {
        id: gcsBatteryPage

        ToolIndicatorPage {
            contentComponent: Component {
                SettingsGroupLayout {
                    popoverStyle: true
                    heading: qsTr("Ground Station")

                    LabelledLabel {
                        label:      qsTr("Charge")
                        labelText:  control._battery.level + "%"
                    }

                    LabelledLabel {
                        label:      qsTr("State")
                        labelText:  control._battery.charging ? qsTr("Charging") : qsTr("On battery")
                    }
                }
            }
        }
    }
}
