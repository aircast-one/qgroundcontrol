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

// The edit-mode corner badge. With a rig and editKey it toggles the item hidden or back;
// without them it is a plain delete badge and the caller handles clicked().
OverlayCapsule {
    id:         _root
    width:      ScreenTools.defaultFontPixelHeight * 1.2
    height:     width
    highlight:  true
    z:          10

    // Sits fully inside its parent: badged items enable a layer for their drop shadow, and a
    // layer crops children to the item's bounds plus whatever padding the effect happens to add.
    anchors.horizontalCenter:       parent.left
    anchors.verticalCenter:         parent.top
    anchors.horizontalCenterOffset: width / 2
    anchors.verticalCenterOffset:   height / 2

    visible: rig ? rig.editMode : false

    property var    rig
    property string editKey: ""

    Component.onCompleted: {
        if (rig && editKey !== "") {
            rig.registerHideKey(editKey)
        }
    }

    readonly property bool hidden: rig && editKey !== "" ? rig.isHidden(editKey) : false

    signal clicked()

    // An eye, not a cross: x reads as "delete this permanently" everywhere else, and users
    // avoid a control they think is destructive. Hiding is reversible - a hidden item stays on
    // screen as a ghost in edit mode with this badge showing the eye back on.
    QGCColoredImage {
        anchors.centerIn:   parent
        width:              parent.width * 0.6
        height:             width
        fillMode:           Image.PreserveAspectFit
        sourceSize.height:  height
        color:              _root.contentColor
        source:             _root.hidden ? "/InstrumentValueIcons/view-show.svg"
                                         : "/InstrumentValueIcons/view-hide.svg"
    }

    // The badge is drawn small so it does not cover the control it sits on, but it is aimed at
    // with a finger: the hit area grows out to the platform's minimum target while the visible
    // capsule stays where it is.
    QGCMouseArea {
        anchors.fill:    parent
        anchors.margins: -Math.max(0, (ScreenTools.minTouchPixels - parent.width) / 2)
        onClicked: {
            if (_root.rig && _root.editKey !== "") {
                _root.rig.setHidden(_root.editKey, !_root.hidden)
            }
            _root.clicked()
        }
    }
}
