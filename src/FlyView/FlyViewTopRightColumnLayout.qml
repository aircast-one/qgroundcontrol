import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlyView
import QGroundControl.FlightMap

ColumnLayout {
    width: _rightPanelWidth

    required property var overlayRig

    TerrainProgress {
        id:                     terrainProgress
        Layout.alignment:       Qt.AlignTop
        Layout.preferredWidth:  _rightPanelWidth

        Component.onCompleted:   overlayRig.registerStatic(terrainProgress)
        Component.onDestruction: overlayRig.unregisterStatic(terrainProgress)
    }

}
