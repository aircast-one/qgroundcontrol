import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.FlightDisplay
import QGroundControl.Palette
import QGroundControl.ScreenTools

Item {
    id: mainWindow

    property string page:       "fly"
    property string toolSource: ""

    readonly property Item  contentItem:            mainWindow
    readonly property Item  header:                 headerStub
    readonly property real  windowChromeLeftInset:  0
    readonly property real  windowChromeRightInset: 0
    readonly property real  panelRadius:            Math.round(ScreenTools.defaultFontPixelHeight * 1.6)
    readonly property bool  toolDrawerVisible:      toolLoader.active
    readonly property bool  glassBackdropVisible:   !toolLoader.active
    readonly property bool  flyViewBackdropVisible: !toolLoader.active
    readonly property bool  flyViewActive:          page !== "plan"
    readonly property bool  hostProvidesNavigation: true
    readonly property string activeVehicleText:     globals.activeVehicle ? globals.activeVehicle.flightMode : qsTr("No vehicle")

    signal navigateRequest(string destination)
    signal armVehicleRequest
    signal forceArmVehicleRequest
    signal disarmVehicleRequest
    signal vtolTransitionToFwdFlightRequest
    signal vtolTransitionToMRFlightRequest
    signal showPreFlightChecklistIfNeeded

    function registerWindowDragExclusion(item) {}
    function allowViewSwitch(previousValidationErrorCount = 0) { return globals.validationErrorCount <= previousValidationErrorCount }
    function showPlanView()                                     { navigateRequest("plan") }
    function showFlyView()                                      { navigateRequest("fly") }
    function showSettingsTool(settingsPageUrl = "")             { navigateRequest("settings") }
    function showCommLinkSettings()                             { navigateRequest("settings") }
    function showVideoSettings()                                { navigateRequest("settings") }
    function showFlyViewSettings()                              { navigateRequest("settings") }
    function showVehicleConfig()                                { navigateRequest("setup") }
    function showVehicleConfigParametersPage()                  { navigateRequest("parameters") }
    function showKnownVehicleComponentConfigPage(component)     { navigateRequest("setup") }
    function showAnalyzeTool()                                  { navigateRequest("analyze") }
    function showTool(toolTitle, source, toolIcon)              { toolSource = source }

    function showMessageDialog(dialogTitle, dialogText, buttons = Dialog.Ok, acceptFunction = null, closeFunction = null) {
        simpleMessageDialogComponent.createObject(mainWindow, { title: dialogTitle, text: dialogText, buttons: buttons, acceptFunction: acceptFunction, closeFunction: closeFunction }).open()
    }

    function showIndicatorDrawer(drawerComponent, indicatorItem) {
        flyView.guidedController.closeAll()
        indicatorDrawer.indicatorItem   = indicatorItem
        indicatorDrawer.sourceComponent = drawerComponent
        indicatorDrawer.open()
    }

    function closeIndicatorDrawer() { indicatorDrawer.close() }

    function showParametersPanel() {
        if (toolLoader.item && toolLoader.item.showParametersPanel) {
            toolLoader.item.showParametersPanel()
        }
    }

    Component.onCompleted: QGroundControl.corePlugin.setupEmbeddedEngine(mainWindow)

    Timer {
        interval:   300
        repeat:     true
        running:    true
        onTriggered: running = !QGroundControl.videoManager.initForItem(mainWindow)
    }

    QtObject {
        id: globals

        readonly property var   activeVehicle:                  QGroundControl.multiVehicleManager.activeVehicle
        readonly property real  defaultTextHeight:              ScreenTools.defaultFontPixelHeight
        readonly property real  defaultTextWidth:               ScreenTools.defaultFontPixelWidth
        readonly property var   planMasterControllerFlyView:    flyView.planController
        readonly property var   guidedControllerFlyView:        flyView.guidedController
        readonly property var   overlayRigFlyView:              flyView.overlayRig
        property int            validationErrorCount:           0
        property bool           commingFromRIDIndicator:        false
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    Item { id: headerStub }

    Rectangle {
        anchors.fill:   parent
        color:          QGroundControl.globalPalette.window
    }

    FlyView {
        id:             flyView
        anchors.fill:   parent
        toolbarVisible: false
        visible:        !toolLoader.active
    }

    PlanView {
        id:             planView
        anchors.fill:   parent
        map:            flyView.mapControl
        planActive:     !mainWindow.flyViewActive
        opacity:        mainWindow.flyViewActive ? 0 : 1
        visible:        opacity > 0 && !toolLoader.active
        enabled:        !mainWindow.flyViewActive

        Behavior on opacity { NumberAnimation { duration: 180; easing.type: Easing.InOutQuad } }
    }

    Loader {
        id:             toolLoader
        anchors.fill:   parent
        active:         mainWindow.toolSource !== ""
        source:         mainWindow.toolSource
        visible:        active

        Rectangle {
            anchors.fill:   parent
            color:          qgcPal.window
            z:              -1
            visible:        toolLoader.active
        }
    }

    Component {
        id: simpleMessageDialogComponent

        QGCSimpleMessageDialog {}
    }

    Popup {
        id:             indicatorDrawer
        modal:          true
        dim:            false
        focus:          true
        closePolicy:    Popup.CloseOnEscape | Popup.CloseOnPressOutside
        padding:        ScreenTools.defaultFontPixelHeight * 0.6
        y:              ScreenTools.defaultFontPixelHeight / 2
        x:              indicatorItem
                            ? Math.max(_margins, Math.min(indicatorItem.mapToItem(mainWindow, 0, 0).x, mainWindow.width - width - _margins))
                            : _margins

        property var    sourceComponent
        property var    indicatorItem
        readonly property real  _margins:       ScreenTools.defaultFontPixelHeight / 4
        readonly property color contentColor:   qgcPal.text

        onClosed: {
            indicatorItem   = undefined
            sourceComponent = undefined
        }

        background: Rectangle {
            color:          qgcPal.window
            radius:         ScreenTools.defaultFontPixelHeight * 0.75
            border.color:   qgcPal.groupBorder
            border.width:   1
        }

        contentItem: Loader {
            id:                 indicatorDrawerLoader
            sourceComponent:    indicatorDrawer.sourceComponent
        }
    }
}
