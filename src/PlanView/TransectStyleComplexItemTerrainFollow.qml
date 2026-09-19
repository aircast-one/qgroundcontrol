import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls

ColumnLayout {
    spacing: _margin
    visible: tabBar.currentIndex === 2

    property var missionItem

    MouseArea {
        id:                     distanceModeRow
        Layout.preferredWidth:  childrenRect.width
        Layout.preferredHeight: childrenRect.height

        onClicked: altModeMenu.openFrom(distanceModeRow)

        AltModeMenu {
            id:              altModeMenu
            objectName:      "transectAltModeMenu"
            currentAltMode:  missionItem.cameraCalc.distanceMode
            rgRemoveModes:   [QGroundControl.AltitudeFrameMixed,
                              ...(missionItem.masterController.controllerVehicle.supports.terrainFrame
                                    ? [] : [QGroundControl.AltitudeFrameTerrain]),
                              ...(missionItem.cameraCalc.isManualCamera
                                    ? [] : [QGroundControl.AltitudeFrameAbsolute])]
            updateAltModeFn: (altMode) => { missionItem.cameraCalc.distanceMode = altMode }
        }

        RowLayout {
            spacing: ScreenTools.defaultFontPixelWidth / 2

            QGCLabel { text: QGroundControl.altitudeFrameShortDescription(missionItem.cameraCalc.distanceMode) }
            QGCColoredImage {
                height:     ScreenTools.defaultFontPixelHeight / 2
                width:      height
                source:     "/res/DropArrow.svg"
                color:      qgcPal.text
            }
        }
    }

    GridLayout {
        Layout.fillWidth:   true
        columnSpacing:      _margin
        rowSpacing:         _margin
        columns:            2
        enabled:            missionItem.cameraCalc.distanceMode === QGroundControl.AltitudeFrameCalcAboveTerrain

        QGCLabel { text: qsTr("Tolerance") }
        FactTextField {
            fact:               missionItem.terrainAdjustTolerance
            Layout.fillWidth:   true
        }

        QGCLabel { text: qsTr("Max Climb Rate") }
        FactTextField {
            fact:               missionItem.terrainAdjustMaxClimbRate
            Layout.fillWidth:   true
        }

        QGCLabel { text: qsTr("Max Descent Rate") }
        FactTextField {
            fact:               missionItem.terrainAdjustMaxDescentRate
            Layout.fillWidth:   true
        }
    }
}
