
import QtQuick

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.FlightMap

TransectStyleComplexItemEditor {
    transectAreaDefinitionComplete: _missionItem.corridorPolyline.isValid
    transectAreaDefinitionHelp:     qsTr("Use the Polyline Tools to create the polyline which defines the corridor.")
    transectValuesHeaderName:       qsTr("Corridor")
    transectValuesComponent:        _transectValuesComponent
    presetsTransectValuesComponent: _transectValuesComponent

    property var _missionItem: missionItem

    Component {
        id: _transectValuesComponent

        PlanGroupCard {
            PlanFactRow {
                text: qsTr("Width")
                fact: _missionItem.corridorWidth
            }

            PlanFactRow {
                text:    qsTr("Turnaround distance")
                fact:    _missionItem.turnAroundDistance
                visible: !forPresets
            }

            PlanSwitchRow {
                text:    qsTr("Images in turnarounds")
                fact:    _missionItem.cameraTriggerInTurnAround
                enabled: _missionItem.hoverAndCaptureAllowed ? !_missionItem.hoverAndCapture.rawValue : true
                visible: !forPresets
            }
        }
    }
}
