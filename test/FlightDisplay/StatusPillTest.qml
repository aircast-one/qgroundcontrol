import QtQuick

import QGroundControl
import QGroundControl.FlightDisplay

Item {
    id:     root
    width:  800
    height: 600

    readonly property alias registeredStatics: rig.statics
    readonly property alias registeredOwners:  rig.owners

    QtObject {
        id: rig

        property var statics:  []
        property var owners:   []
        property bool editMode: false

        function registerStatic(item, owner) {
            statics = [...statics, item]
            owners  = [...owners, owner]
        }
        function unregisterStatic(item) { statics = statics.filter((entry) => entry !== item) }
        function registerMovable(item, dragPosition) { }
        function unregisterMovable(item) { }
        function isHidden(key) { return false }
        function requestReflow() { }
    }

    Item {
        id:         pip
        objectName: "pip"
        width:      200
        height:     120
    }

    FlyViewVideo {
        id:             video
        objectName:     "flyViewVideo"
        anchors.fill:   parent
        overlayRig:     rig
        pipView:        pip
    }
}
