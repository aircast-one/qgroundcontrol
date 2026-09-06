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

Item {
    id:     root
    width:  300
    height: 60

    property int  backingIndex:     0

    property int  activatedCount:   0
    property int  lastActivated:    -1
    property int  reselectedCount:  0
    property int  lastReselected:   -1

    readonly property int  currentIndex: viewSwitch.currentIndex
    readonly property real thumbX:       thumb.x

    readonly property Item thumb: viewSwitch.children.find(child => child.objectName === "viewSwitchThumb")

    OverlayViewSwitch {
        id:           viewSwitch
        objectName:   "viewSwitch"
        anchors.fill: parent
        currentIndex: root.backingIndex

        options: [ qsTr("Fly"), qsTr("Plan") ]

        onActivated: (index) => {
            root.activatedCount++
            root.lastActivated = index
        }

        onReselected: (index) => {
            root.reselectedCount++
            root.lastReselected = index
        }
    }
}
