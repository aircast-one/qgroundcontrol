import QtQuick

import QGroundControl
import QGroundControl.Controls

OverlaySegmentedControl {
    segments:    [ qsTr("Grid"), qsTr("Camera"), qsTr("Terrain"), qsTr("Presets") ]
    onActivated: (index) => currentIndex = index

    Component.onCompleted: currentIndex = QGroundControl.settingsManager.planViewSettings.displayPresetsTabFirst.rawValue ? 3 : 0
}
