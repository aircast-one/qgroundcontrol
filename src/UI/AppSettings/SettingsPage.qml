/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/


import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager
import QGroundControl.Palette

Item {
    id: root

    default property alias contentItem: mainLayout.data

    // What the host needs to size a floating panel around this page.
    readonly property real contentWidth:  mainLayout.width
    readonly property real contentHeight: mainLayout.height

    readonly property real _minContentWidth: ScreenTools.defaultFontPixelWidth * 50

    QGCFlickable {
        anchors.fill:   parent
        contentWidth:   mainLayout.width
        contentHeight:  mainLayout.height

        ColumnLayout {
            id:         mainLayout
            x:          0
            // No upper cap. implicitWidth is already the natural width of the content - nothing
            // in here stretches to fill - so a cap does not prevent over-wide rows, it clips
            // pages that are legitimately wide. Remote ID is two columns and lost its second
            // one to exactly that. The host clamps the panel to the window instead, and this
            // Flickable scrolls whatever does not fit.
            width:      Math.max(implicitWidth, _minContentWidth)
            spacing:    ScreenTools.defaultFontPixelHeight
        }
    }
}
