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
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.ScreenTools

PlanGroupRow {
    id: _root

    property alias fact:         field.fact
    property alias altitudeMode: field.altitudeMode

    signal updated()

    TextMetrics {
        id:   fieldMetrics
        font: field.font
        text: field.text + " " + field.unitsLabel + " " + field.extraUnitsLabel
    }

    AltitudeFactTextField {
        id:                     field
        anchors.verticalCenter: parent.verticalCenter
        width:                  Math.max(ScreenTools.defaultFontPixelWidth * 13, fieldMetrics.width + ScreenTools.defaultFontPixelWidth * 4)
        showFrame:              false
        horizontalAlignment:    TextInput.AlignRight
        onUpdated:              _root.updated()
    }
}
