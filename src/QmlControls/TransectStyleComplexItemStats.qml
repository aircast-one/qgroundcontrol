import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls

Grid {
    columns:        2
    columnSpacing:  ScreenTools.defaultFontPixelWidth

    readonly property var _units: QGroundControl.unitsConversion

    function _measure(value, unit) {
        return Number(value).toLocaleString(Qt.locale(), "f", value >= 100 ? 0 : 1) + " " + unit.replace("^2", "\u00B2")
    }

    QGCLabel { text: qsTr("Area") }
    QGCLabel { text: _measure(_units.squareMetersToAppSettingsAreaUnits(missionItem.coveredArea), _units.appSettingsAreaUnitsString) }

    QGCLabel { text: qsTr("Distance") }
    QGCLabel { text: _measure(_units.metersToAppSettingsHorizontalDistanceUnits(missionItem.complexDistance), _units.appSettingsHorizontalDistanceUnitsString) }

    QGCLabel { text: qsTr("Photos") }
    QGCLabel { text: missionItem.cameraShots }

    QGCLabel { text: qsTr("Photo interval") }
    QGCLabel { text: qsTr("%1 s").arg(missionItem.timeBetweenShots.toFixed(1)) }
}
