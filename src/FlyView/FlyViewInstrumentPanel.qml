import QtQuick

import QGroundControl
import QGroundControl.Controls

SelectableControl {
    z:                      QGroundControl.zOrderWidgets
    selectedControl:        QGroundControl.settingsManager.flyViewSettings.instrumentQmlFile2

    property var  missionController:    _missionController
    property real extraInset:           innerControl.extraInset
}
