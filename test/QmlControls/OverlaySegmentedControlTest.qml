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

    // Stands in for whatever the owner drives the control from - in the plan view, the layer
    // index. The control must follow it for as long as the owner keeps the binding.
    property int  backingIndex:   0
    property bool thirdEnabled:   false

    property int  activatedCount: 0
    property int  lastActivated:  -1

    readonly property int currentIndex: segmented.currentIndex

    OverlaySegmentedControl {
        id:           segmented
        objectName:   "segmented"
        anchors.fill: parent
        currentIndex: root.backingIndex

        segments: [
            { text: "One",   enabled: true },
            { text: "Two",   enabled: true },
            { text: "Three", enabled: root.thirdEnabled }
        ]

        onActivated: (index) => {
            root.activatedCount++
            root.lastActivated = index
        }
    }
}
