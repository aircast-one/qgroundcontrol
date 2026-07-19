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
        property var pipState: _aState
        PipState { id: _aState; pipView: pip }
    }

    Rectangle {
        id: itemB
        objectName: "itemB"
        color: "blue"
        property var pipState: _bState
        PipState { id: _bState; pipView: pip }
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
