import QtQuick

import QGroundControl
import QGroundControl.FlightDisplay

Item {
    id: root
    width: 800
    height: 600

    Item {
        id: pip
        objectName: "pip"
        x: 20
        y: 400
        width: naturalWidth
        height: width * 9 / 16
        property bool expanded: true
        property real widthOverride: 0
        readonly property real naturalWidth: widthOverride > 0 ? widthOverride : 300
    }

    VideoTilesLayer {
        id: tiles
        objectName: "tiles"
        anchors.fill: parent
        pipView: pip
    }

    Binding { target: pip; property: "widthOverride"; value: tiles.pipWidthOverride }
}
