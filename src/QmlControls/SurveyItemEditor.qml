
import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.FlightMap

TransectStyleComplexItemEditor {
    transectAreaDefinitionComplete: missionItem.surveyAreaPolygon.isValid
    transectAreaDefinitionHelp:     qsTr("Use the Polygon Tools to create the polygon which outlines your survey area.")
    transectValuesHeaderName:       qsTr("Transects")
    transectValuesComponent:        _transectValuesComponent
    presetsTransectValuesComponent: _transectValuesComponent
    entryPointText:                 qsTr("Start from")
    entryPointValue:                _entryNames[missionItem.entryPoint]

    property var _missionItem: missionItem
    property var _vehicle:     QGroundControl.multiVehicleManager.activeVehicle ? QGroundControl.multiVehicleManager.activeVehicle : QGroundControl.multiVehicleManager.offlineEditingVehicle

    readonly property var _entryNames: [ qsTr("top left"), qsTr("top right"), qsTr("bottom left"), qsTr("bottom right") ]

    Component {
        id: _transectValuesComponent

        PlanGroupCard {
            PlanFactRow {
                text:      qsTr("Angle")
                fact:      missionItem.gridAngle
                onUpdated: angleSlider.value = missionItem.gridAngle.value
            }

            Item {
                width:  parent.width
                height: ScreenTools.defaultFontPixelHeight * 2

                QGCSlider {
                    id:                     angleSlider
                    anchors.left:           parent.left
                    anchors.right:          parent.right
                    anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    from:                   0
                    to:                     359
                    stepSize:               1
                    tickmarksEnabled:       false
                    live:                   true
                    onValueChanged:         missionItem.gridAngle.value = value
                    Component.onCompleted:  value = missionItem.gridAngle.value
                }
            }

            PlanFactRow {
                text:    qsTr("Turnaround distance")
                fact:    missionItem.turnAroundDistance
                visible: !forPresets
            }

            PlanSwitchRow {
                text:    qsTr("Hover to capture each image")
                fact:    missionItem.hoverAndCapture
                enabled: missionItem.cameraCalc.distanceMode === QGroundControl.AltitudeModeRelative || missionItem.cameraCalc.distanceMode === QGroundControl.AltitudeModeAbsolute
                visible: !forPresets && missionItem.hoverAndCaptureAllowed
            }

            PlanSwitchRow {
                text:    qsTr("Refly at 90° for a cross grid")
                fact:    missionItem.refly90Degrees
                enabled: missionItem.cameraCalc.distanceMode !== QGroundControl.AltitudeModeCalcAboveTerrain
                visible: !forPresets
            }

            PlanSwitchRow {
                text:    qsTr("Images in turnarounds")
                fact:    missionItem.cameraTriggerInTurnAround
                enabled: missionItem.hoverAndCaptureAllowed ? !missionItem.hoverAndCapture.rawValue : true
                visible: !forPresets
            }

            PlanSwitchRow {
                text:    qsTr("Fly alternate transects")
                fact:    missionItem.flyAlternateTransects
                visible: !forPresets && (_vehicle ? (_vehicle.fixedWing || _vehicle.vtol) : false)
            }
        }
    }

    KMLOrSHPFileDialog {
        id:             kmlOrSHPLoadDialog
        title:          qsTr("Select Polygon File")

        onAcceptedForLoad: (file) => {
            missionItem.surveyAreaPolygon.loadKMLOrSHPFile(file)
            missionItem.resetState = false
            close()
        }
    }
}
