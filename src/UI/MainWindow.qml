/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls
import QtQuick.Dialogs
import QtQuick.Effects
import QtQuick.Layouts
import QtQuick.Window

import QGroundControl
import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.ScreenTools
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap

import QGroundControl.UTMSP

ApplicationWindow {
    id:             mainWindow
    visible:        true

    property bool   _utmspSendActTrigger
    property bool   _utmspStartTelemetry

    Component.onCompleted: {
        firstRunPromptManager.nextPrompt()
    }

    MainWindowSavedState {
        window: mainWindow
    }

    QtObject {
        id: firstRunPromptManager

        property var currentDialog:     null
        property var rgPromptIds:       QGroundControl.corePlugin.firstRunPromptsToShow()
        property int nextPromptIdIndex: 0

        function clearNextPromptSignal() {
            if (currentDialog) {
                currentDialog.closed.disconnect(nextPrompt)
            }
        }

        function nextPrompt() {
            if (nextPromptIdIndex < rgPromptIds.length) {
                var component = Qt.createComponent(QGroundControl.corePlugin.firstRunPromptResource(rgPromptIds[nextPromptIdIndex]));
                currentDialog = component.createObject(mainWindow)
                currentDialog.closed.connect(nextPrompt)
                currentDialog.open()
                nextPromptIdIndex++
            } else {
                currentDialog = null
                showPreFlightChecklistIfNeeded()
            }
        }
    }

    readonly property real      _topBottomMargins:          ScreenTools.defaultFontPixelHeight * 0.5

    readonly property real windowChromeLeftInset:  windowChromeLoader.item ? windowChromeLoader.item.leftInset : 0
    readonly property real windowChromeRightInset: windowChromeLoader.item ? windowChromeLoader.item.rightInset : 0

    readonly property bool glassBackdropVisible: flyView.visible && !toolDrawer.visible
    readonly property bool flyViewBackdropVisible: flyView.visible
    readonly property real panelRadius: Math.round(ScreenTools.defaultFontPixelHeight * 1.6)

    property var _windowDragExclusions: []

    function registerWindowDragExclusion(item) {
        _windowDragExclusions = [..._windowDragExclusions, item]
        if (windowChromeLoader.item) {
            windowChromeLoader.item.excludeFromDrag(item)
        }
    }


    QtObject {
        id: globals

        readonly property var       activeVehicle:                  QGroundControl.multiVehicleManager.activeVehicle
        readonly property real      defaultTextHeight:              ScreenTools.defaultFontPixelHeight
        readonly property real      defaultTextWidth:               ScreenTools.defaultFontPixelWidth
        readonly property var       planMasterControllerFlyView:    flyView.planController
        readonly property var       guidedControllerFlyView:        flyView.guidedController
        readonly property var       overlayRigFlyView:              flyView.overlayRig

        property int                validationErrorCount:           0 

        property bool               commingFromRIDIndicator:        false
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }


    signal armVehicleRequest
    signal forceArmVehicleRequest
    signal disarmVehicleRequest
    signal vtolTransitionToFwdFlightRequest
    signal vtolTransitionToMRFlightRequest
    signal showPreFlightChecklistIfNeeded


    function allowViewSwitch(previousValidationErrorCount = 0) {
        if (mainWindow.activeFocusControl instanceof FactTextField) {
            mainWindow.activeFocusControl._onEditingFinished()
        }
        return globals.validationErrorCount <= previousValidationErrorCount
    }
    property bool flyViewActive: true

    function showPlanView() {
        flyViewActive = false
    }

    function showFlyView() {
        flyViewActive = true
    }

    function showTool(toolTitle, toolSource, toolIcon) {
        toolDrawer.backIcon     = flyView.visible ? "/qmlimages/PaperPlane.svg" : "/qmlimages/Plan.svg"
        toolDrawer.toolTitle    = toolTitle
        toolDrawer.toolSource   = toolSource
        toolDrawer.toolIcon     = toolIcon
        toolDrawer.visible      = true
    }

    function showAnalyzeTool() {
        showTool(qsTr("Analyze Tools"), "qrc:/qml/QGroundControl/AnalyzeView/AnalyzeView.qml", "/qmlimages/Analyze.svg")
    }

    function showVehicleConfig() {
        showTool(qsTr("Vehicle Configuration"), "qrc:/qml/QGroundControl/VehicleSetup/SetupView.qml", "/qmlimages/Gears.svg")
    }

    function showVehicleConfigParametersPage() {
        showVehicleConfig()
        toolDrawerLoader.item.showParametersPanel()
    }

    function showKnownVehicleComponentConfigPage(knownVehicleComponent) {
        showVehicleConfig()
        let vehicleComponent = globals.activeVehicle.autopilotPlugin.findKnownVehicleComponent(knownVehicleComponent)
        if (vehicleComponent) {
            toolDrawerLoader.item.showVehicleComponentPanel(vehicleComponent)
        }
    }

    function showSettingsTool(settingsPage = "") {
        showTool(qsTr("Settings"), "qrc:/qml/QGroundControl/Controls/AppSettings.qml", "/res/QGCLogoWhite")
        if (settingsPage !== "") {
            toolDrawerLoader.item.showSettingsPage(settingsPage)
        }
    }


    function showMessageDialog(dialogTitle, dialogText, buttons = Dialog.Ok, acceptFunction = null, closeFunction = null) {
        simpleMessageDialogComponent.createObject(mainWindow, { title: dialogTitle, text: dialogText, buttons: buttons, acceptFunction: acceptFunction, closeFunction: closeFunction }).open()
    }

    function _showMessageDialog(dialogTitle, dialogText) {
        showMessageDialog(dialogTitle, dialogText)
    }

    Component {
        id: simpleMessageDialogComponent

        QGCSimpleMessageDialog {
        }
    }

    property bool _forceClose: false

    function finishCloseProcess() {
        _forceClose = true
        firstRunPromptManager.clearNextPromptSignal()
        QGroundControl.linkManager.shutdown()
        QGroundControl.videoManager.stopVideo();
        mainWindow.close()
    }

    readonly property int _skipUnsavedMissionCheckMask: 0x01
    readonly property int _skipPendingParameterWritesCheckMask: 0x02
    readonly property int _skipActiveConnectionsCheckMask: 0x04
    property int _closeChecksToSkip: 0
    function performCloseChecks() {
        if (!(_closeChecksToSkip & _skipUnsavedMissionCheckMask) && !checkForUnsavedMission()) {
            return false
        }
        if (!(_closeChecksToSkip & _skipPendingParameterWritesCheckMask) && !checkForPendingParameterWrites()) {
            return false
        }
        if (!(_closeChecksToSkip & _skipActiveConnectionsCheckMask) && !checkForActiveConnections()) {
            return false
        }
        finishCloseProcess()
        return true
    }

    property string closeDialogTitle: qsTr("Close %1").arg(QGroundControl.appName)

    function checkForUnsavedMission() {
        if (planView._planMasterController.dirty) {
            showMessageDialog(closeDialogTitle,
                              qsTr("You have a mission edit in progress which has not been saved/sent. If you close you will lose changes. Are you sure you want to close?"),
                              Dialog.Yes | Dialog.No,
                              function() { _closeChecksToSkip |= _skipUnsavedMissionCheckMask; performCloseChecks() })
            return false
        } else {
            return true
        }
    }

    function checkForPendingParameterWrites() {
        for (var index=0; index<QGroundControl.multiVehicleManager.vehicles.count; index++) {
            if (QGroundControl.multiVehicleManager.vehicles.get(index).parameterManager.pendingWrites) {
                mainWindow.showMessageDialog(closeDialogTitle,
                    qsTr("You have pending parameter updates to a vehicle. If you close you will lose changes. Are you sure you want to close?"),
                    Dialog.Yes | Dialog.No,
                    function() { _closeChecksToSkip |= _skipPendingParameterWritesCheckMask; performCloseChecks() })
                return false
            }
        }
        return true
    }

    function checkForActiveConnections() {
        if (QGroundControl.multiVehicleManager.activeVehicle) {
            mainWindow.showMessageDialog(closeDialogTitle,
                qsTr("There are still active connections to vehicles. Are you sure you want to exit?"),
                Dialog.Yes | Dialog.No,
                function() { _closeChecksToSkip |= _skipActiveConnectionsCheckMask; performCloseChecks() })
            return false
        } else {
            return true
        }
    }

    onClosing: (close) => {
        if (!_forceClose) {
            _closeChecksToSkip = 0
            close.accepted = performCloseChecks()
        }
    }

    background: Rectangle {
        anchors.fill:   parent
        color:          QGroundControl.globalPalette.window
    }
    FlyView {
        id:                     flyView
        anchors.fill:           parent
        utmspSendActTrigger:    _utmspSendActTrigger
    }

    PlanView {
        id:             planView
        anchors.fill:   parent
        map:            flyView.mapControl
        planActive:     !mainWindow.flyViewActive
        opacity:        mainWindow.flyViewActive ? 0 : 1
        visible:        opacity > 0
        enabled:        !mainWindow.flyViewActive

        Behavior on opacity { NumberAnimation { duration: 220; easing.type: Easing.InOutQuad } }
    }

    footer: LogReplayStatusBar {
        visible: QGroundControl.settingsManager.flyViewSettings.showLogReplayStatusBar.rawValue
    }

    MessageDialog {
        id:                 showTouchAreasNotification
        title:              qsTr("Debug Touch Areas")
        text:               qsTr("Touch Area display toggled")
        buttons:            MessageDialog.Ok
    }

    MessageDialog {
        id:                 advancedModeOnConfirmation
        title:              qsTr("Advanced Mode")
        text:               QGroundControl.corePlugin.showAdvancedUIMessage
        buttons:            MessageDialog.Yes | MessageDialog.No
        onButtonClicked: function (button, role) {
            if (button === MessageDialog.Yes) {
                QGroundControl.corePlugin.showAdvancedUI = true
            }
        }
    }

    MessageDialog {
        id:                 advancedModeOffConfirmation
        title:              qsTr("Advanced Mode")
        text:               qsTr("Turn off Advanced Mode?")
        buttons:            MessageDialog.Yes | MessageDialog.No
        onButtonClicked: function (button, role) {
            if (button === MessageDialog.Yes) {
                QGroundControl.corePlugin.showAdvancedUI = false
            }
        }
    }

    Rectangle {
        id:             toolDrawer
        anchors.fill:   parent
        visible:        false
        color:          floating ? Qt.rgba(0, 0, 0, 0.55) : qgcPal.window

        property var backIcon
        property string toolTitle
        property alias toolSource:  toolDrawerLoader.source
        property var toolIcon

        readonly property var  _tool:    toolDrawerLoader.item
        readonly property real _toolWidth:  _tool && _tool.preferredWidth  > 0 ? _tool.preferredWidth  : 0
        readonly property real _toolHeight: _tool && _tool.preferredHeight > 0 ? _tool.preferredHeight : 0
        readonly property bool floating: _toolWidth > 0 && _toolHeight > 0

        readonly property real _panelWidth:  Math.min(_toolWidth,
                                                      width - ScreenTools.defaultFontPixelWidth * 4)
        readonly property real _panelHeight: Math.min(_toolHeight + toolDrawerToolbar.height,
                                                      height - ScreenTools.defaultFontPixelHeight * 2)

        onVisibleChanged: {
            if (!toolDrawer.visible) {
                toolDrawerLoader.source = ""
            }
        }

        DeadMouseArea {
            anchors.fill: parent
        }

        Rectangle {
            id:                 toolPanel
            objectName:         "toolPanel"
            anchors.centerIn:   parent
            width:              toolDrawer.floating ? toolDrawer._panelWidth  : parent.width
            height:             toolDrawer.floating ? toolDrawer._panelHeight : parent.height
            color:              toolDrawer.floating ? "transparent" : qgcPal.window
            radius:             toolDrawer.floating ? mainWindow.panelRadius : 0
            clip:               true

            OverlayGlass {
                anchors.fill:   parent
                radius:         toolPanel.radius
                visible:        toolDrawer.floating
                frosted:        mainWindow.flyViewBackdropVisible
                material:       OverlayGlass.Panel
            }

            layer.enabled:      toolDrawer.floating
            layer.effect:       MultiEffect {
                maskEnabled:        true
                maskSource:         toolPanelMask
                maskThresholdMin:   0.5
                maskSpreadAtMin:    1.0
            }

            Item {
                id:             toolPanelMask
                anchors.fill:   parent
                layer.enabled:  true
                visible:        false

                Rectangle {
                    anchors.fill:   parent
                    radius:         toolPanel.radius
                    color:          "black"
                }
            }

            Behavior on width  { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }
            Behavior on height { NumberAnimation { duration: 200; easing.type: Easing.OutCubic } }

        Rectangle {
            id:             toolDrawerToolbar
            anchors.left:   parent.left
            anchors.right:  parent.right
            anchors.top:    parent.top
            height:         ScreenTools.toolbarHeight
            color:          toolDrawer.floating ? "transparent" : qgcPal.toolbarBackground

            RowLayout {
                id:                 toolDrawerToolbarLayout
                anchors.leftMargin: ScreenTools.defaultFontPixelWidth +
                                        (toolDrawer.floating ? 0 : mainWindow.windowChromeLeftInset)
                anchors.left:       parent.left
                anchors.top:        parent.top
                anchors.bottom:     parent.bottom
                spacing:            ScreenTools.defaultFontPixelWidth

                QGCColoredImage {
                    Layout.preferredWidth:  ScreenTools.defaultFontPixelHeight
                    Layout.preferredHeight: ScreenTools.defaultFontPixelHeight
                    source:                 "/InstrumentValueIcons/cheveron-left.svg"
                    sourceSize.height:      height
                    color:                  qgcPal.colorBlue
                }

                QGCLabel {
                    id:             toolbarDrawerText
                    text:           toolDrawer.toolTitle
                    font.pointSize: ScreenTools.largeFontPointSize
                    font.bold:      true
                }
            }

            QGCMouseArea {
                anchors.fill:           toolDrawerToolbarLayout
                Component.onCompleted:  mainWindow.registerWindowDragExclusion(this)
                onClicked: {
                    if (mainWindow.allowViewSwitch()) {
                        toolDrawer.visible = false
                    }
                }
            }
        }

        Loader {
            id:             toolDrawerLoader
            anchors.left:   parent.left
            anchors.right:  parent.right
            anchors.top:    toolDrawerToolbar.bottom
            anchors.bottom: parent.bottom

            Connections {
                target:                 toolDrawerLoader.item
                ignoreUnknownSignals:   true
                onPopout:               toolDrawer.visible = false
            }
        }
        }
    }

    Loader {
        id:             windowChromeLoader
        anchors.fill:   parent
        z:              QGroundControl.zOrderTopMost + 1
        active:         !ScreenTools.isMobile
        source:         "WindowChrome.qml"
        onLoaded:       mainWindow._windowDragExclusions.forEach(exclusion => item.excludeFromDrag(exclusion))
    }


    function showCriticalVehicleMessage(message) {
        closeIndicatorDrawer()
        if (criticalVehicleMessagePopup.visible || QGroundControl.videoManager.fullScreen) {
            criticalVehicleMessagePopup.additionalCriticalMessagesReceived = true
        } else {
            criticalVehicleMessagePopup.criticalVehicleMessage      = message
            criticalVehicleMessagePopup.additionalCriticalMessagesReceived = false
            criticalVehicleMessagePopup.open()
        }
    }

    Popup {
        id:                 criticalVehicleMessagePopup
        y:                  ScreenTools.toolbarHeight + ScreenTools.defaultFontPixelHeight
        x:                  Math.round((mainWindow.width - width) * 0.5)
        width:              mainWindow.width  * 0.55
        height:             criticalVehicleMessageText.contentHeight + ScreenTools.defaultFontPixelHeight * 2
        modal:              false
        focus:              true

        property alias  criticalVehicleMessage:             criticalVehicleMessageText.text
        property bool   additionalCriticalMessagesReceived: false

        background: Rectangle {
            anchors.fill:   parent
            color:          qgcPal.alertBackground
            radius:         ScreenTools.defaultFontPixelHeight * 0.5
            border.color:   qgcPal.alertBorder
            border.width:   2

            Rectangle {
                anchors.horizontalCenter:   parent.horizontalCenter
                anchors.top:                parent.top
                anchors.topMargin:          -(height / 2)
                color:                      qgcPal.alertBackground
                radius:                     ScreenTools.defaultFontPixelHeight * 0.25
                border.color:               qgcPal.alertBorder
                border.width:               1
                width:                      vehicleWarningLabel.contentWidth + _margins
                height:                     vehicleWarningLabel.contentHeight + _margins

                property real _margins: ScreenTools.defaultFontPixelHeight * 0.25

                QGCLabel {
                    id:                 vehicleWarningLabel
                    anchors.centerIn:   parent
                    text:               qsTr("Vehicle Error")
                    font.pointSize:     ScreenTools.smallFontPointSize
                    color:              qgcPal.alertText
                }
            }

            Rectangle {
                id:                         additionalErrorsIndicator
                anchors.horizontalCenter:   parent.horizontalCenter
                anchors.bottom:             parent.bottom
                anchors.bottomMargin:       -(height / 2)
                color:                      qgcPal.alertBackground
                radius:                     ScreenTools.defaultFontPixelHeight * 0.25
                border.color:               qgcPal.alertBorder
                border.width:               1
                width:                      additionalErrorsLabel.contentWidth + _margins
                height:                     additionalErrorsLabel.contentHeight + _margins
                visible:                    criticalVehicleMessagePopup.additionalCriticalMessagesReceived

                property real _margins: ScreenTools.defaultFontPixelHeight * 0.25

                QGCLabel {
                    id:                 additionalErrorsLabel
                    anchors.centerIn:   parent
                    text:               qsTr("Additional errors received")
                    font.pointSize:     ScreenTools.smallFontPointSize
                    color:              qgcPal.alertText
                }
            }
        }

        QGCLabel {
            id:                 criticalVehicleMessageText
            width:              criticalVehicleMessagePopup.width - ScreenTools.defaultFontPixelHeight
            anchors.centerIn:   parent
            wrapMode:           Text.WordWrap
            color:              qgcPal.alertText
            textFormat:         TextEdit.RichText
        }

        MouseArea {
            anchors.fill: parent
            onClicked: {
                criticalVehicleMessagePopup.close()
                if (criticalVehicleMessagePopup.additionalCriticalMessagesReceived) {
                    criticalVehicleMessagePopup.additionalCriticalMessagesReceived = false;
                    flyView.dropMainStatusIndicatorTool();
                } else {
                    QGroundControl.multiVehicleManager.activeVehicle.resetErrorLevelMessages();
                }
            }
        }
    }


    function showIndicatorDrawer(drawerComponent, indicatorItem) {
        flyView.guidedController.closeAll()
        indicatorDrawer.sourceComponent = drawerComponent
        indicatorDrawer.indicatorItem = indicatorItem
        indicatorDrawer.open()
    }

    function closeIndicatorDrawer() {
        indicatorDrawer.close()
    }

    Popup {
        id:             indicatorDrawer
        objectName:     "indicatorDrawer"
        x: {
            if (!indicatorItem) {
                return _margins
            }
            const left = indicatorItem.mapToItem(mainWindow.contentItem, 0, 0).x
            const maxX = mainWindow.contentItem.width - width - _margins
            return Math.max(_margins, Math.min(left, maxX))
        }
        y:              ScreenTools.toolbarHeight + (ScreenTools.defaultFontPixelHeight / 2)
        leftInset:      0
        rightInset:     0
        topInset:       0
        bottomInset:    0
        padding:        ScreenTools.defaultFontPixelHeight * 0.6
        visible:        false
        modal:          true
        dim:            false
        focus:          true
        closePolicy:    Popup.CloseOnEscape | Popup.CloseOnPressOutside

        property var sourceComponent
        property var indicatorItem
        readonly property color contentColor: qgcPal.text

        property bool _expanded:    false
        property real _margins:     ScreenTools.defaultFontPixelHeight / 4

        readonly property real _panelRadius: ScreenTools.defaultFontPixelHeight * 0.75
        readonly property real _innerRadius: Math.max(0, _panelRadius - padding)

        property real _morph: 0

        readonly property point _sourceCentre: indicatorItem
            ? indicatorItem.mapToItem(mainWindow.contentItem, indicatorItem.width / 2, indicatorItem.height / 2)
            : Qt.point(x + width / 2, y)
        readonly property real _originX: Math.max(0, Math.min(width,  _sourceCentre.x - x))
        readonly property real _originY: Math.max(0, Math.min(height, _sourceCentre.y - y))
        readonly property real _startScale: indicatorItem && width > 0
            ? Math.max(0.12, Math.min(0.9, indicatorItem.width / width))
            : 0.3
        readonly property real _scale: _startScale + (1 - _startScale) * _morph

        enter: Transition {
            NumberAnimation { target: indicatorDrawer; property: "_morph"; from: 0; to: 1
                              duration: 260; easing.type: Easing.OutCubic }
            NumberAnimation { property: "opacity"; from: 0; to: 1; duration: 130 }
        }

        exit: Transition {
            NumberAnimation { target: indicatorDrawer; property: "_morph"; from: 1; to: 0
                              duration: 170; easing.type: Easing.InCubic }
            NumberAnimation { property: "opacity"; from: 1; to: 0; duration: 170 }
        }

        onOpened: {
            _expanded                               = false;
            indicatorDrawerLoader.sourceComponent   = indicatorDrawer.sourceComponent
        }
        onClosed: {
            _expanded                               = false
            indicatorItem                           = undefined
            indicatorDrawerLoader.sourceComponent   = undefined
        }

        Rectangle {
            parent:         mainWindow.contentItem
            visible:        indicatorDrawer.visible && indicatorDrawer.indicatorItem
            radius:         ScreenTools.defaultFontPixelHeight * 0.4
            color:          Qt.alpha(QGroundControl.globalPalette.text, 0.15)
            z:              indicatorDrawer.z - 1

            readonly property point _origin: indicatorDrawer.indicatorItem
                ? indicatorDrawer.indicatorItem.mapToItem(mainWindow.contentItem, 0, 0)
                : Qt.point(0, 0)
            readonly property real _pad: ScreenTools.defaultFontPixelWidth * 0.6

            x:      _origin.x - _pad
            y:      _origin.y - _pad / 2
            width:  (indicatorDrawer.indicatorItem ? indicatorDrawer.indicatorItem.width  : 0) + _pad * 2
            height: (indicatorDrawer.indicatorItem ? indicatorDrawer.indicatorItem.height : 0) + _pad
        }

        background: Item {
            transform: Scale {
                origin.x: indicatorDrawer._originX
                origin.y: indicatorDrawer._originY
                xScale:   indicatorDrawer._scale
                yScale:   indicatorDrawer._scale
            }

            Rectangle {
                id:             backgroundRect
                anchors.fill:   parent
                color:          "transparent"
                radius:         indicatorDrawer._panelRadius
                layer.enabled:  true
                layer.effect:   OverlayShadowEffect { }

                OverlayGlass {
                    objectName:    "drawerGlass"
                    anchors.fill:  parent
                    radius:        parent.radius
                    frosted:       false
                    lightMaterial: false
                }
            }

        }

        contentItem: ColumnLayout {
            spacing: indicatorDrawer._margins
            opacity: indicatorDrawer._morph * indicatorDrawer._morph

            transform: Scale {
                origin.x: indicatorDrawer._originX - indicatorDrawer.padding
                origin.y: indicatorDrawer._originY - indicatorDrawer.padding
                xScale:   indicatorDrawer._scale
                yScale:   indicatorDrawer._scale
            }

            QGCFlickable {
                id:                 indicatorDrawerLoaderFlickable
                Layout.fillWidth:   true
                implicitWidth:  Math.min(mainWindow.contentItem.width - (2 * indicatorDrawer._margins) - (indicatorDrawer.padding * 2), indicatorDrawerLoader.width)
                implicitHeight: Math.min(mainWindow.contentItem.height - ScreenTools.toolbarHeight - (2 * indicatorDrawer._margins) - (indicatorDrawer.padding * 2), indicatorDrawerLoader.height)
                contentWidth:   indicatorDrawerLoader.width
                contentHeight:  indicatorDrawerLoader.height

                Loader {
                    id: indicatorDrawerLoader

                    Binding {
                        target:     indicatorDrawerLoader.item
                        property:   "expanded"
                        value:      indicatorDrawer._expanded
                    }

                    Binding {
                        target:     indicatorDrawerLoader.item
                        property:   "drawer"
                        value:      indicatorDrawer
                    }
                }
            }

            Rectangle {
                Layout.fillWidth:       true
                Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 2
                implicitWidth:          detailsLabel.implicitWidth + detailsChevron.width + ScreenTools.defaultFontPixelWidth * 4
                radius:                 indicatorDrawer._innerRadius
                color:                  moreArea.containsMouse ? Qt.alpha(QGroundControl.globalPalette.text, 0.08)
                                                               : "transparent"
                visible:                indicatorDrawerLoader.item &&
                                            indicatorDrawerLoader.item.showExpand && !indicatorDrawer._expanded

                QGCLabel {
                    id:                     detailsLabel
                    anchors.left:           parent.left
                    anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    text:                   indicatorDrawerLoader.item ? indicatorDrawerLoader.item.expandText : ""
                }

                QGCColoredImage {
                    id:                     detailsChevron
                    anchors.right:          parent.right
                    anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    source:                 "/InstrumentValueIcons/cheveron-right.svg"
                    color:                  QGroundControl.globalPalette.colorGrey
                    height:                 ScreenTools.defaultFontPixelHeight * 0.9
                    width:                  height
                    sourceSize.height:      height
                    fillMode:               Image.PreserveAspectFit
                    mipmap:                 true
                }

                QGCMouseArea {
                    id:             moreArea
                    anchors.fill:   parent
                    hoverEnabled:   !ScreenTools.isMobile
                    onClicked:      indicatorDrawer._expanded = true
                }
            }
        }
    }


    function createrWindowedAnalyzePage(title, source) {
        var windowedPage = windowedAnalyzePage.createObject(mainWindow)
        windowedPage.title = title
        windowedPage.source = source
    }

    Component {
        id: windowedAnalyzePage

        Window {
            width:      ScreenTools.defaultFontPixelWidth  * 100
            height:     ScreenTools.defaultFontPixelHeight * 40
            visible:    true

            property alias source: loader.source

            Rectangle {
                color:          QGroundControl.globalPalette.window
                anchors.fill:   parent

                Loader {
                    id:             loader
                    anchors.fill:   parent
                    onLoaded:       item.popped = true
                }
            }

            onClosing: {
                visible = false
                source = ""
            }
        }
    }

    Connections{
         target: activationbar
         function onActivationTriggered(value){
              _utmspSendActTrigger= value
         }
    }

    UTMSPActivationStatusBar{
         id:                         activationbar
         activationStartTimestamp:   UTMSPStateStorage.startTimeStamp
         activationApproval:         UTMSPStateStorage.showActivationTab && QGroundControl.utmspManager.utmspVehicle.vehicleActivation
         flightID:                   UTMSPStateStorage.flightID
         anchors.fill:               parent
    }
}
