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
import QGroundControl.ScreenTools

OverlayCapsule {
    id: _root

    property real   progress: -1
    property string text
    property bool   done:     false

    readonly property real _pad: ScreenTools.defaultFontPixelWidth * 1.6

    width: ring.width + label.implicitWidth + _pad * 2 + ScreenTools.defaultFontPixelWidth

    Behavior on width { NumberAnimation { duration: 160; easing.type: Easing.OutCubic } }

    OverlayActivityRing {
        id:                     ring
        anchors.left:           parent.left
        anchors.leftMargin:     _root._pad
        anchors.verticalCenter: parent.verticalCenter
        progress:               _root.progress
        done:                   _root.done
        contentColor:           _root.contentColor
    }

    QGCLabel {
        id:                     label
        anchors.left:           ring.right
        anchors.leftMargin:     ScreenTools.defaultFontPixelWidth
        anchors.verticalCenter: parent.verticalCenter
        text:                   _root.text
        color:                  _root.contentColor
    }
}
