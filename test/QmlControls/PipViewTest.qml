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
    id: root
    width: 800
    height: 600

    Rectangle {
        id: itemA
        objectName: "itemA"
        color: "red"
        property var pipState: aState
        PipState { id: aState; pipView: pip }
    }

    Rectangle {
        id: itemB
        objectName: "itemB"
        color: "blue"
        property var pipState: bState
        PipState { id: bState; pipView: pip }
    }

    PipView {
        id: pip
        objectName: "pip"
        margin: 8
        item1: itemA
        item2: itemB
        item1IsFullSettingsKey: "PipViewTestItem1IsFull"
    }
}
