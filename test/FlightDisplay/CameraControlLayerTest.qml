import QtQuick

import QGroundControl
import QGroundControl.FlightDisplay

Item {
    width: 900
    height: 600

    QtObject {
        id: stubOverlayRig

        property bool editMode: false

        function registerStatic(item) { }
    }

    CameraControlLayer {
        objectName:     "cameraControlLayer"
        anchors.fill:   parent
        visible:        true
        overlayRig:     stubOverlayRig
        videoIsMainItem: true
    }
}
