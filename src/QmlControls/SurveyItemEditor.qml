import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Vehicle
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
    entryPointText:                 qsTr("Start from %1").arg(_entryNames[missionItem.entryPoint])


    property real   _margin:        ScreenTools.defaultFontPixelWidth / 2
    property var    _missionItem:   missionItem

    readonly property var _entryNames: [ qsTr("top left"), qsTr("top right"), qsTr("bottom left"), qsTr("bottom right") ]

    Component {
        id: _transectValuesComponent

        GridLayout {
            Layout.fillWidth:   true
            columnSpacing:      _margin
            rowSpacing:         _margin
            columns:            2

            QGCLabel { text: qsTr("Angle") }
            FactTextField {
                fact:                   missionItem.gridAngle
                Layout.fillWidth:       true
                onUpdated:              angleSlider.value = missionItem.gridAngle.value
            }

            QGCSlider {
                id:                     angleSlider
                from:           0
                to:           359
                stepSize:               1
                tickmarksEnabled:       false
                Layout.fillWidth:       true
                Layout.columnSpan:      2
                Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 1.5
                onValueChanged:         missionItem.gridAngle.value = value
                Component.onCompleted:  value = missionItem.gridAngle.value
                live: true
            }

            QGCLabel {
                text:       qsTr("Turnaround dist")
                visible:    !forPresets
            }
            FactTextField {
                Layout.fillWidth:   true
                fact:               missionItem.turnAroundDistance
                visible:            !forPresets
            }

            FactCheckBox {
                Layout.columnSpan:  2
                text:               qsTr("Hover to capture each image")
                fact:               missionItem.hoverAndCapture
                enabled:            missionItem.cameraCalc.distanceMode === QGroundControl.AltitudeModeRelative || missionItem.cameraCalc.distanceMode === QGroundControl.AltitudeModeAbsolute
                visible:            !forPresets && missionItem.hoverAndCaptureAllowed
            }

            FactCheckBox {
                Layout.columnSpan:  2
                text:               qsTr("Refly at 90\u00B0 for a cross grid")
                fact:               missionItem.refly90Degrees
                enabled:            missionItem.cameraCalc.distanceMode !== QGroundControl.AltitudeModeCalcAboveTerrain
                visible:            !forPresets
            }

            FactCheckBox {
                Layout.columnSpan:  2
                text:               qsTr("Images in turnarounds")
                fact:               missionItem.cameraTriggerInTurnAround
                enabled:            missionItem.hoverAndCaptureAllowed ? !missionItem.hoverAndCapture.rawValue : true
                visible:            !forPresets
            }

            FactCheckBox {
                Layout.columnSpan:  2
                text:               qsTr("Fly alternate transects")
                fact:               missionItem.flyAlternateTransects
                visible:            !forPresets && (_vehicle ? (_vehicle.fixedWing || _vehicle.vtol) : false)
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
