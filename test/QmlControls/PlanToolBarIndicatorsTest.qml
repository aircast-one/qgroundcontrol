import QtQuick
import QtPositioning

import QGroundControl.Controls
import QGroundControl.Controllers
import QGroundControl.ScreenTools

Item {
    id:     root
    width:  360
    height: 80

    readonly property real fontPixelWidth: ScreenTools.defaultFontPixelWidth

    function addWaypoint() {
        const mission = controller.missionController
        mission.insertSimpleMissionItem(QtPositioning.coordinate(47.4, 8.5), mission.visualItems.count, true)
    }

    QtObject {
        id: mainWindow

        property bool flyViewActive: false

        function allowViewSwitch() { return true }
        function showPlanView() { }
        function showFlyView() { }
        function showIndicatorDrawer() { }
        function closeIndicatorDrawer() { }
        function showMessageDialog() { }
    }

    PlanMasterController {
        id:      controller
        flyView: false

        Component.onCompleted: start()
    }

    PlanToolBarIndicators {
        objectName:           "planRow"
        width:                root.width
        planMasterController: controller
    }
}
