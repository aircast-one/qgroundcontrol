import QtQuick

import QGroundControl
import QGroundControl.Controls

Rectangle {
    id: _root

    default property alias rows: rowColumn.data

    color:          QGroundControl.globalPalette.windowShade
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
