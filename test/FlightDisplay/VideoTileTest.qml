import QtQuick

import QGroundControl
import QGroundControl.FlightDisplay

Item {
    id: root
    width: 800
    height: 600

    Item { id: pipStub }

    FlyViewVideo {
        id: video
        objectName: "video"
        anchors.fill: parent
        pipView: pipStub
    }
}
