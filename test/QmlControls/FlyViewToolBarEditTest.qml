import QtQuick

import QGroundControl
import QGroundControl.Controls

Item {
    id:     root
    width:  1200
    height: 200

    property alias editMode:    stubRig.editMode
    property int   drawerCount: 0

    function setHidden(key, hidden) { stubRig.setHidden(key, hidden) }

    QtObject {
        id: stubRig

        readonly property real hiddenOpacity: 0.35

        property bool editMode: false
        property var  heldItem: null
        property var  hidden:   ({})

        readonly property int dragThreshold: 32767

        function isHidden(key)          { return hidden[key] === true }
        function registerHideKey(key)   { }
        function setHidden(key, value) {
            const next = Object.assign({}, hidden)
            next[key] = value
            hidden = next
        }
        function hold(item)                             { editMode = true }
        function requestReflow()                        { }
        function registerMovable(item, dragPosition)    { }
        function unregisterMovable(item)                { }
        function registerStatic(item, owner)            { }
        function unregisterStatic(item)                 { }
        function resetLayout()                          { }
    }

    QtObject {
        id: globals

        property var overlayRigFlyView: stubRig
        property var activeVehicle:     null
    }

    QtObject {
        id: mainWindow

        property bool flyViewActive:         true
        property real windowChromeLeftInset:  0
        property real windowChromeRightInset: 0

        function registerWindowDragExclusion(item)  { }
        function allowViewSwitch()                  { return true }
        function showFlyView()                      { }
        function showPlanView()                     { }
        function showAnalyzeTool()                  { }
        function showSettingsTool()                 { }
        function showVehicleConfig()                { }
        function showMessageDialog()                { }
        function closeIndicatorDrawer()             { }
        function showIndicatorDrawer(component, control) { root.drawerCount++ }
    }

    FlyViewToolBar {
        objectName: "flyViewToolBar"
        overlayRig: stubRig
    }
}
