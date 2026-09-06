import QtQuick
import QtPositioning

import QGroundControl.Controls
import QGroundControl.FlightMap
import QGroundControl.ScreenTools

Item {
    id:     root
    width:  428
    height: 903

    readonly property real fontPixelWidth:       ScreenTools.defaultFontPixelWidth
    readonly property real terrainProfileHeight: planView._terrainProfileHeight

    function addWaypoint() {
        const mission = planView._missionController
        mission.insertSimpleMissionItem(map.center, mission.visualItems.count, true)
    }

    QtObject {
        id: mainWindow

        property bool flyViewActive:         false
        property real windowChromeLeftInset:  0
        property real windowChromeRightInset: 0

        function registerWindowDragExclusion(item) { }
        function allowViewSwitch() { return true }
        function showMessageDialog() { }
        function showPlanView() { }
        function showFlyView() { }
        function showIndicatorDrawer() { }
        function closeIndicatorDrawer() { }
    }

    FlightMap {
        id:           map
        anchors.fill: parent
        center:       QtPositioning.coordinate(47.4, 8.5)
        zoomLevel:    10
    }

    PlanView {
        id:           planView
        objectName:   "planView"
        anchors.fill: parent
        map:          map
        planActive:   true
    }
}
