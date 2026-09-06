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
import QtQuick.Layouts
import QtQuick.Dialogs

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Controllers

// A shelf of floating controls, not a bar. The solid strip it replaced ended the map at its
// bottom edge and made the plan look like it was being viewed through a window rather than
// worked on directly.
Item {
    id:     _root
    width:  parent.width
    height: toolRow.height + _margin * 2

    property var  planMasterController
    // How much of the right edge the inspector already owns. The shelf stops there rather than
    // running under it, so the primary action never sits on top of the panel.
    property real rightInset: 0

    readonly property real _margin: ScreenTools.defaultFontPixelHeight * 0.9

    Component.onCompleted: mainWindow.registerWindowDragExclusion(toolRow)

    PlanToolBarIndicators {
        id:                     toolRow
        anchors.top:            parent.top
        anchors.topMargin:      _margin + ScreenTools.safeAreaTop
        anchors.left:           parent.left
        anchors.right:          parent.right
        anchors.leftMargin:     _margin + ScreenTools.safeAreaLeft + mainWindow.windowChromeLeftInset
        anchors.rightMargin:    _margin + ScreenTools.safeAreaRight + Math.max(mainWindow.windowChromeRightInset, _root.rightInset)
        planMasterController:   _root.planMasterController
    }
}
