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
        x:          leftItem.x + leftItem.width + 5
        y:          leftItem.y
        width:      80
        height:     60

        Component.onCompleted: overlayRig.registerStatic(attachedItem, leftItem)
    }

    Rectangle {
        id:         carrierItem
        objectName: "carrierItem"
        width:      120
        height:     60

        DragToPosition {
            id:                 carrierPosition
            objectName:         "carrierPosition"
            target:             carrierItem
            settingsKeyPrefix:  "OverlayRigTestCarrier"
            defaultX:           60
            defaultY:           700
        }

        Component.onCompleted: overlayRig.registerMovable(carrierItem, carrierPosition)
    }

    Rectangle {
        id:         carrierRail
        objectName: "carrierRail"
        x:          carrierItem.x + carrierItem.width + 5
        y:          carrierItem.y
        width:      80
        height:     60

        Component.onCompleted: overlayRig.registerStatic(carrierRail, carrierItem)
    }

    Rectangle {
        id:         passengerItem
        objectName: "passengerItem"
        width:      90
        height:     50

        DragToPosition {
            id:                 passengerPosition
            objectName:         "passengerPosition"
            target:             passengerItem
            settingsKeyPrefix:  "OverlayRigTestPassenger"
            defaultX:           150
            defaultY:           700
        }

        Component.onCompleted: overlayRig.registerMovable(passengerItem, passengerPosition)
    }
}
