/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQml.Models

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlightDisplay
import QGroundControl.ScreenTools

ToolStrip {
    id: _root

    color:          "transparent"
    roundButtons:   true
    buttonSpacing:  ScreenTools.defaultFontPixelHeight / 2
    width:          Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.4) +
                        ScreenTools.defaultFontPixelWidth * 0.8

    signal displayPreFlightChecklist

    FlyViewToolStripActionList {
        id: flyViewToolStripActionList

        onDisplayPreFlightChecklist: _root.displayPreFlightChecklist()
    }

    model: flyViewToolStripActionList.model
}
