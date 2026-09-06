/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap
import QGroundControl.Palette
import QGroundControl.ScreenTools

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
