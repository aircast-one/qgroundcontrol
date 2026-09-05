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
import QtQuick.Dialogs
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.Controllers

Item {
    id:             setupView
    enabled:        !_disableDueToArmed && !_disableDueToFlying

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    property alias  pageComponent:          pageLoader.sourceComponent
    property string pageName:               vehicleComponent ? vehicleComponent.name : ""
    property string pageDescription:        vehicleComponent ? vehicleComponent.description : ""
    property real   availableWidth:         width - pageLoader.x
    property real   availableHeight:        height - pageLoader.y
    property bool   showAdvanced:           false
    property alias  advanced:               advancedCheckBox.checked

    property bool   _vehicleIsRover:        globals.activeVehicle ? globals.activeVehicle.rover : false
    property bool   _vehicleArmed:          globals.activeVehicle ? globals.activeVehicle.armed : false
    property bool   _vehicleFlying:         globals.activeVehicle ? globals.activeVehicle.flying : false
    property bool   _disableDueToArmed:     vehicleComponent ? (!vehicleComponent.allowSetupWhileArmed && _vehicleArmed) : false
    property bool   _disableDueToFlying:    vehicleComponent ? (!_vehicleIsRover && !vehicleComponent.allowSetupWhileFlying && _vehicleFlying) : false
    property string _disableReason:         _disableDueToArmed ? qsTr("armed") : qsTr("flying")
    property real   _margins:               ScreenTools.defaultFontPixelHeight * 0.5

    Component.onCompleted: {
        if(pageLoader.item && pageLoader.item.setupPageCompleted) {
            pageLoader.item.setupPageCompleted()
        }
    }

    QGCFlickable {
        anchors.fill:   parent
        contentWidth:   Math.max(availableWidth, pageLoader.x + pageLoader.item.width)
        contentHeight:  Math.max(availableHeight, pageLoader.y + pageLoader.item.height)
        clip:           true

        RowLayout {
            id:                 headingRow
            width:              availableWidth
            spacing:            _margins
            layoutDirection:    Qt.RightToLeft
            visible:            showAdvanced || !setupView.enabled || (pageDescription !== "" && !ScreenTools.isShortScreen)

            QGCCheckBox {
                id:         advancedCheckBox
                text:       qsTr("Advanced")
                visible:    showAdvanced
            }

            ColumnLayout {
                spacing:            _margins / 2
                Layout.fillWidth:   true

                QGCLabel {
                    Layout.fillWidth:   true
                    wrapMode:           Text.WordWrap
                    text:               pageDescription
                    color:              Qt.alpha(qgcPal.text, 0.6)
                    visible:            pageDescription !== "" && !ScreenTools.isShortScreen
                }

                QGCLabel {
                    Layout.fillWidth:   true
                    wrapMode:           Text.WordWrap
                    text:               qsTr("Disabled while the vehicle is %1").arg(_disableReason)
                    color:              qgcPal.colorOrange
                    font.bold:          true
                    visible:            !setupView.enabled
                }
            }
        }

        Loader {
            id:                 pageLoader
            anchors.topMargin:  _margins
            anchors.top:        headingRow.bottom
        }

        Rectangle {
            visible:            !setupView.enabled
            anchors.fill:       parent
            color:              "black"
            opacity:            0.5
        }
    }
}
