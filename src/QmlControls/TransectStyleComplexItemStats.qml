import QtQuick

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls

PlanGroupCard {
    readonly property var _units: QGroundControl.unitsConversion

    function _measure(value, unit) {
        return Number(value).toLocaleString(Qt.locale(), "f", value >= 100 ? 0 : 1) + " " + unit.replace("^2", "²")
    }

    PlanGroupRow { text: qsTr("Area");           value: _measure(_units.squareMetersToAppSettingsAreaUnits(missionItem.coveredArea), _units.appSettingsAreaUnitsString) }
    PlanGroupRow { text: qsTr("Distance");       value: _measure(_units.metersToAppSettingsHorizontalDistanceUnits(missionItem.complexDistance), _units.appSettingsHorizontalDistanceUnitsString) }
    PlanGroupRow { text: qsTr("Photos");         value: String(missionItem.cameraShots) }
    PlanGroupRow { text: qsTr("Photo interval"); value: qsTr("%1 s").arg(missionItem.timeBetweenShots.toFixed(1)) }
}
