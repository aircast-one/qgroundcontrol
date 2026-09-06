import QtQuick

import QGroundControl
import QGroundControl.FlightDisplay

Item {
    width: 900
    height: 600

    QtObject {
        id: stubOverlayRig

        readonly property real hiddenOpacity: 0.35

        property bool editMode: false

        function registerStatic(item, owner) { }
        function unregisterStatic(item) { }
        function registerMovable(item, dragPosition) { }
        function registerAnchor(item, dragPosition) { }
        function unregisterMovable(item) { }
        function registerHideKey(key) { }
        function isHidden(key) { return false }
        function setHidden(key, hidden) { }
        function requestReflow() { }
        function resolve(item) { }
    }

    CameraControlLayer {
        objectName:     "cameraControlLayer"
        anchors.fill:   parent
        visible:        true
        overlayRig:     stubOverlayRig
        videoIsMainItem: true
    }
}
