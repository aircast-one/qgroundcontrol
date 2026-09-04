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
    width:  1000
    height: 800

    property alias rig: overlayRig

    OverlayRig {
        id:             overlayRig
        viewport:       root
    }

    Rectangle {
        id:         leftItem
        objectName: "leftItem"
        x:          20
        y:          400
        width:      100
        height:     60

        DragToPosition {
            id:                 leftPosition
            objectName:         "leftPosition"
            target:             leftItem
            settingsKeyPrefix:  "OverlayRigTestLeft"
            defaultX:           20
            defaultY:           400
        }

        Component.onCompleted: overlayRig.registerMovable(leftItem, leftPosition)
    }

    Rectangle {
        id:         rightItem
        objectName: "rightItem"
        width:      100
        height:     60

        DragToPosition {
            id:                 rightPosition
            objectName:         "rightPosition"
            target:             rightItem
            settingsKeyPrefix:  "OverlayRigTestRight"
            defaultX:           root.width - rightItem.width - 20
            defaultY:           600
        }

        Component.onCompleted: overlayRig.registerMovable(rightItem, rightPosition)
    }

    // An order of magnitude heavier than leftItem: mass is area, so the pair must separate
    // almost entirely by moving the light one.
    Rectangle {
        id:         heavyItem
        objectName: "heavyItem"
        width:      250
        height:     200

        DragToPosition {
            id:                 heavyPosition
            objectName:         "heavyPosition"
            target:             heavyItem
            settingsKeyPrefix:  "OverlayRigTestHeavy"
            defaultX:           150
            defaultY:           60
        }

        Component.onCompleted: overlayRig.registerMovable(heavyItem, heavyPosition)
    }

    Rectangle {
        id:         staticItem
        objectName: "staticItem"
        x:          500
        y:          100
        width:      80
        height:     80

        Component.onCompleted: overlayRig.registerStatic(staticItem)
    }

    Rectangle {
        id:         attachedItem
        objectName: "attachedItem"
        x:          125
        y:          400
        width:      80
        height:     60

        Component.onCompleted: overlayRig.registerStatic(attachedItem, leftItem)
    }
}
