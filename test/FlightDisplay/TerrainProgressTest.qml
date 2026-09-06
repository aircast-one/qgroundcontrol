import QtQuick

import QGroundControl.FlightDisplay

Item {
    id:     root
    width:  600
    height: 400

    readonly property alias statics: stubRig.statics

    property real _rightPanelWidth: 240

    QtObject {
        id: stubRig

        property int statics: 0

        function registerStatic(item, owner) { statics++ }
        function unregisterStatic(item)      { statics-- }
    }

    FlyViewTopRightColumnLayout {
        objectName: "topRightColumn"
        overlayRig: stubRig
    }
}
