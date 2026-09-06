/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.Controls
import QGroundControl.ScreenTools

OverlayCapsule {
    id: _root

    property alias text: pillLabel.text

    signal clicked()

    width:     pillLabel.width + ScreenTools.defaultFontPixelWidth * 4
    highlight: pillMouseArea.containsMouse

    QGCLabel {
        id:                 pillLabel
        anchors.centerIn:   parent
        color:              _root.contentColor
    }

    QGCMouseArea {
        id:             pillMouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
        onClicked:      _root.clicked()
    }
}
