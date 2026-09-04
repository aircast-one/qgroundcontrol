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
import QGroundControl.ScreenTools
import QGroundControl.Controls

Rectangle {
    height:             contentRow.height + _padding * 1.5
    width:              contentRow.width + _padding * 3
    color:              _qgcPal.overlayBackground
    border.color:       _qgcPal.overlayBorder
    border.width:       1
    radius:             ScreenTools.defaultFontPixelHeight * 0.75
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { }
    visible:            _noGPSLockVisible || _prearmErrorVisible

    property var  _activeVehicle:       QGroundControl.multiVehicleManager.activeVehicle
    property bool _noGPSLockVisible:    _activeVehicle && _activeVehicle.requiresGpsFix && !_activeVehicle.coordinate.isValid
    property bool _prearmErrorVisible:  _activeVehicle && !_activeVehicle.armed && _activeVehicle.prearmError && !_activeVehicle.healthAndArmingCheckReport.supported

    property var  _qgcPal:              QGroundControl.globalPalette
    property real _padding:             ScreenTools.defaultFontPixelHeight * 0.75

    Row {
        id:                 contentRow
        anchors.centerIn:   parent
        spacing:            _padding * 0.75

        QGCColoredImage {
            anchors.verticalCenter: parent.verticalCenter
            source:                 "/qmlimages/Yield.svg"
            color:                  _qgcPal.colorOrange
            height:                 ScreenTools.defaultFontPixelHeight * 1.2
            width:                  height
            sourceSize.height:      height
            fillMode:               Image.PreserveAspectFit
            mipmap:                 true
            smooth:                 true
        }

        Column {
            anchors.verticalCenter: parent.verticalCenter
            spacing:                ScreenTools.defaultFontPixelHeight / 4

            QGCLabel {
                visible:    _noGPSLockVisible
                color:      _qgcPal.text
                text:       qsTr("No GPS Lock for Vehicle")
            }

            QGCLabel {
                visible:    _prearmErrorVisible
                color:      _qgcPal.text
                text:       _activeVehicle ? _activeVehicle.prearmError : ""
            }

            QGCLabel {
                visible:    _prearmErrorVisible
                width:      ScreenTools.defaultFontPixelWidth * 50
                wrapMode:   Text.WordWrap
                color:      _qgcPal.text
                opacity:    0.7
                font.pointSize: ScreenTools.smallFontPointSize
                text:       qsTr("The vehicle has failed a pre-arm check. In order to arm the vehicle, resolve the failure.")
            }
        }
    }
}
