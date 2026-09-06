import QtQuick

import QGroundControl
import QGroundControl.FlightDisplay

Item {
    width:  900
    height: 600

    property alias editMode:   stubOverlayRig.editMode
    property alias resetCount: stubOverlayRig.resetCount

    function setSlotGrid(grid) { stubOverlayRig.positions.forEach((position) => { position.snapGrid = grid }) }

    QtObject {
        id: stubOverlayRig

        property bool editMode:   false
        property int  resetCount: 0

        property var positions: []

        function registerMovable(item, dragPosition) { positions = [...positions, dragPosition] }
        function unregisterMovable(item) { }
        function requestReflow() { }
        function isHidden(key) { return false }
        function registerHideKey(key) { }
        function setHidden(key, hidden) { }
        function resetLayout() { resetCount++ }
    }

    TelemetryChipsLayer {
        objectName:     "telemetryChipsLayer"
        anchors.fill:   parent
        overlayRig:     stubOverlayRig
    }
}
