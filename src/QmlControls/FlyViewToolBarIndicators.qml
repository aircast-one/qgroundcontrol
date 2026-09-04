/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Toolbar

//-------------------------------------------------------------------------
//-- Toolbar Indicators
// Height comes from the instantiation site: the indicators size their icons from it, so the
// bar decides how large they render instead of the strip stretching to fill it.
Row {
    id:                 indicatorRow
    spacing:            ScreenTools.defaultFontPixelWidth * 1.75

    property var  _activeVehicle:           QGroundControl.multiVehicleManager.activeVehicle

    Repeater {
        id:     appRepeater
        model:  QGroundControl.corePlugin.toolBarIndicators
        Loader {
            anchors.top:        parent.top
            anchors.bottom:     parent.bottom
            source:             modelData
            visible:            item.showIndicator
        }
    }

    Repeater {
        id:     toolIndicatorsRepeater
        model:  _activeVehicle ? _activeVehicle.toolIndicators : []

        // Flight mode lives in the toolbar's fixed status cluster, not the scrolling strip
        Loader {
            anchors.top:        parent.top
            anchors.bottom:     parent.bottom
            source:             modelData
            visible:            item.showIndicator && modelData.toString().indexOf("FlightModeIndicator") < 0
        }
    }

    Repeater {
        model: _activeVehicle ? _activeVehicle.modeIndicators : []
        Loader {
            anchors.top:        parent.top
            anchors.bottom:     parent.bottom
            source:             modelData
            visible:            item.showIndicator
        }
    }
}
