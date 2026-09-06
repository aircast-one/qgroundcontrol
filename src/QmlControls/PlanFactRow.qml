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
    property alias field:        field

    AltitudeFactTextField {
        id:                     field
        anchors.verticalCenter: parent.verticalCenter
        width:                  ScreenTools.defaultFontPixelWidth * 13
        showFrame:              false
        horizontalAlignment:    TextInput.AlignRight
    }
}
