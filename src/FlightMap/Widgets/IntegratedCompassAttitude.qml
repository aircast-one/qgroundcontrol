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
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap

Item {
    id:             control
    implicitWidth:  compassRadius * 2
    implicitHeight: implicitWidth

    property real extraInset:                   0
    property real extraValuesWidth:             compassRadius
    property real defaultCompassRadius:         (mainWindow.width * 0.15) / 2
    property real maxCompassRadius:             ScreenTools.defaultFontPixelHeight * 7 / 2
    property real compassRadius:                Math.min(defaultCompassRadius, maxCompassRadius)
    property real compassBorder:                ScreenTools.defaultFontPixelHeight / 2
    property var  vehicle:                      globals.activeVehicle
    property var  qgcPal:                       QGroundControl.globalPalette
    property bool usedByMultipleVehicleList:    false

    Rectangle {
        width:  compassRadius * 2
        height: width
        radius: width / 2
        color:  qgcPal.overlayBackground
        border.color:   qgcPal.overlayBorder
        border.width:   1
        layer.enabled: true
        layer.effect:  OverlayShadowEffect { }

        QGCCompassWidget {
            size:                       parent.width - compassBorder
            vehicle:                    control.vehicle
            usedByMultipleVehicleList:  control.usedByMultipleVehicleList
            anchors.centerIn:           parent
        }
    }
}
