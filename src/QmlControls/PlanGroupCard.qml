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

// An inset group of rows on the inspector panel. Rows sit flush inside one surface separated by
// hairlines, rather than each carrying its own border and shadow - a list of framed boxes reads
// as a list of unrelated things.
Rectangle {
    id: _root

    default property alias rows: rowColumn.data

    color:          Qt.alpha(QGroundControl.globalPalette.text, 0.055)
    radius:         ScreenTools.defaultFontPixelHeight * 0.9
    implicitHeight: rowColumn.implicitHeight
    clip:           true

    Column {
        id:             rowColumn
        anchors.left:   parent.left
        anchors.right:  parent.right
        anchors.top:    parent.top
    }
}
