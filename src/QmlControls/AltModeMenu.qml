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
import QGroundControl.ScreenTools

// Picks the frame altitudes are measured in. Opens beside the row that shows the current mode,
// marks that mode, and dims the ones the mission or vehicle will not accept rather than hiding
// them - a mode that vanishes reads as a bug, a mode that is greyed reads as a rule.
OverlayPopover {
    id: _root

    property var updateAltModeFn
    property int currentAltMode:  QGroundControl.AltitudeModeNone
    property var rgRemoveModes:   []
    property var rgDisableModes:  []

    readonly property real _rowWidth: ScreenTools.defaultFontPixelWidth * 38

    readonly property var _allModes: [
        {
            value: QGroundControl.AltitudeModeRelative,
            name:  qsTr("Relative To Launch"),
            help:  qsTr("Above the launch position.")
        },
        {
            value: QGroundControl.AltitudeModeAbsolute,
            name:  qsTr("AMSL"),
            help:  qsTr("Above mean sea level.")
        },
        {
            value: QGroundControl.AltitudeModeCalcAboveTerrain,
            name:  qsTr("Calculated Above Terrain"),
            help:  qsTr("Above terrain, converted to AMSL before upload.")
        },
        {
            value: QGroundControl.AltitudeModeTerrainFrame,
            name:  qsTr("Terrain Frame"),
            help:  qsTr("Above terrain, held by the vehicle in flight.")
        },
        {
            value: QGroundControl.AltitudeModeMixed,
            name:  qsTr("Mixed Modes"),
            help:  qsTr("Each item sets its own.")
        }
    ]

    readonly property var _modes: _allModes.filter((mode) =>
        _root.rgRemoveModes.indexOf(mode.value) === -1 &&
        (mode.value !== QGroundControl.AltitudeModeAbsolute ||
            QGroundControl.corePlugin.options.showMissionAbsoluteAltitude ||
            mode.value === _root.currentAltMode))

    Repeater {
        model: _root._modes

        OverlayMenuItem {
            Layout.preferredWidth:  _root._rowWidth
            objectName:             "altMode" + modelData.value
            text:                   modelData.name
            description:            modelData.help
            checkable:              true
            checked:                modelData.value === _root.currentAltMode
            enabled:                checked || _root.rgDisableModes.indexOf(modelData.value) === -1
            opacity:                enabled ? 1 : 0.45

            onClicked: {
                _root.close()
                _root.updateAltModeFn(modelData.value)
            }
        }
    }
}
