import QtQuick

import QGroundControl.ScreenTools

Item {
    id: root

    width:  400
    height: 300

    property real barHeight: 60

    readonly property real topEdgeInset: Math.max(0, barHeight - ScreenTools.safeAreaTop)

    Item {
        id: chrome
        objectName: "chrome"

        anchors.fill:           parent
        anchors.leftMargin:     ScreenTools.safeAreaLeft
        anchors.rightMargin:    ScreenTools.safeAreaRight
        anchors.topMargin:      ScreenTools.safeAreaTop
        anchors.bottomMargin:   ScreenTools.safeAreaBottom

        Item {
            objectName: "backdrop"

            anchors.fill:           parent
            anchors.leftMargin:     -ScreenTools.safeAreaLeft
            anchors.rightMargin:    -ScreenTools.safeAreaRight
            anchors.topMargin:      -ScreenTools.safeAreaTop
            anchors.bottomMargin:   -ScreenTools.safeAreaBottom
        }
    }
}
