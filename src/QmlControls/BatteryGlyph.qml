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

// The one battery in the app. The ground station and every vehicle pack draw this, so two
// batteries side by side are the same object showing different states rather than two
// drawings that happen to look similar.
//
// Colour is state, never decoration: neutral while it is fine, and a colour only when it
// wants attention. A battery that goes green at full charge spends the one signal it has.
Item {
    id: _root

    property real fill:     0       // 0..1, NaN-safe via the caller
    property bool charging: false
    property color color:   QGroundControl.globalPalette.toolbarText

    // Deliberately larger than a menu-bar glyph: this is read at arm's length in sunlight, not
    // glanced at on a desk. The proportions follow the system - hairline stroke, matched to the
    // type beside it - but the scale is set by the field, not by the desktop.
    height: ScreenTools.defaultFontPixelHeight * 1.15
    width:  Math.round(height * 1.9)

    readonly property real _clamped: Math.max(0, Math.min(1, fill))

    Rectangle {
        id:             shell
        anchors.left:   parent.left
        width:          parent.width - nub.width
        height:         parent.height
        radius:         height * 0.3
        color:          "transparent"
        border.color:   _root.color
        // Hairline. A 2-3px outline next to hairline system glyphs reads as bold, and the fill
        // inside it is what carries the value anyway.
        border.width:   Math.max(1, Math.round(parent.height * 0.07))

        Rectangle {
            anchors.left:           parent.left
            anchors.leftMargin:     shell.border.width * 2
            anchors.verticalCenter: parent.verticalCenter
            height:                 parent.height - shell.border.width * 4
            width:                  (parent.width - shell.border.width * 4) * _root._clamped
            radius:                 height * 0.25
            color:                  _root.color

            Behavior on width { NumberAnimation { duration: 300; easing.type: Easing.OutCubic } }
        }
    }

    Rectangle {
        id:                     nub
        anchors.left:           shell.right
        anchors.verticalCenter: parent.verticalCenter
        width:                  parent.height * 0.14
        height:                 parent.height * 0.42
        radius:                 width / 2
        color:                  _root.color
    }

    QGCLabel {
        anchors.centerIn:   shell
        text:               "⚡"
        color:              QGroundControl.globalPalette.toolbarText
        font.pointSize:     ScreenTools.smallFontPointSize
        visible:            _root.charging
    }
}
