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
import QtQuick.Layouts

import QGroundControl
import QGroundControl.AutoPilotPlugin
import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager

ToolDrawerPage {
    id:         setupView
    objectName: "vehicleSetupView"
    color:      qgcPal.window
    z:          QGroundControl.zOrderTopMost

    DeadMouseArea {
        anchors.fill: parent
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    readonly property real      _defaultTextHeight: ScreenTools.defaultFontPixelHeight
    readonly property real      _defaultTextWidth:  ScreenTools.defaultFontPixelWidth
    readonly property real      _horizontalMargin:  _defaultTextWidth * 1.25
    readonly property real      _verticalMargin:    _defaultTextHeight / 2
    readonly property real      _inset:             Math.round(_defaultTextHeight * 0.75)
    readonly property real      _sidebarWidth:      _defaultTextWidth * 26
    readonly property real      _sectionGap:        _defaultTextHeight * 0.75

    property string _messagePanelText:              qsTr("missing message panel text")
    property string _prerequisiteName:              ""
    property bool   _fullParameterVehicleAvailable: QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable && !QGroundControl.multiVehicleManager.activeVehicle.parameterManager.missingParameters
    property var    _corePlugin:                    QGroundControl.corePlugin
    property bool   _setupComplete:                 _fullParameterVehicleAvailable ? QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.setupComplete : true
    property string _latestStableFirmware:          QGroundControl.multiVehicleManager.activeVehicle ? QGroundControl.multiVehicleManager.activeVehicle.latestStableFirmwareVersion : ""
    property bool   _firmwareUpdateAvailable:       _latestStableFirmware !== ""
    property bool   _parametersAvailable:           QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable &&
                                                    !QGroundControl.multiVehicleManager.activeVehicle.usingHighLatencyLink &&
                                                    _corePlugin.showAdvancedUI

    readonly property string _search:       searchField.text.trim().toLowerCase()
    readonly property bool   _hasTuning:    _fullParameterVehicleAvailable &&
                                            QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.vehicleComponents
                                                .some(component => isTuning(component) && component.setupSource.toString() !== "")
    sidebarWidth:    sidebar.width
    preferredWidth:  ScreenTools.isMobile ? 0 : Math.max(_defaultTextWidth * 110, mainWindow.width * 0.82)
    preferredHeight: ScreenTools.isMobile ? 0 : Math.max(_defaultTextHeight * 34, mainWindow.height * 0.86)

    readonly property var    _componentStyles: ({
        "/qmlimages/AirframeComponentIcon.png":    { icon: "/InstrumentValueIcons/box.svg",              color: "#ff9f0a" },
        "/qmlimages/SubFrameComponentIcon.png":    { icon: "/InstrumentValueIcons/box.svg",              color: "#ff9f0a" },
        "/res/helicoptericon.svg":                 { icon: "/InstrumentValueIcons/box.svg",              color: "#ff9f0a" },
        "/qmlimages/RadioComponentIcon.png":       { icon: "/InstrumentValueIcons/radio.svg",            color: "#0a84ff" },
        "/qmlimages/FlightModesComponentIcon.png": { icon: "/InstrumentValueIcons/swap.svg",             color: "#5e5ce6" },
        "/qmlimages/SensorsComponentIcon.png":     { icon: "/InstrumentValueIcons/radar.svg",            color: "#30d158" },
        "/qmlimages/PowerComponentIcon.png":       { icon: "/InstrumentValueIcons/bolt.svg",             color: "#ff375f" },
        "/qmlimages/MotorComponentIcon.svg":       { icon: "/InstrumentValueIcons/refresh.svg",          color: "#bf5af2" },
        "/qmlimages/SafetyComponentIcon.png":      { icon: "/InstrumentValueIcons/shield.svg",           color: "#ff453a" },
        "/qmlimages/TuningComponentIcon.png":      { icon: "/InstrumentValueIcons/tuning.svg",           color: "#ac8e68" },
        "/qmlimages/CameraComponentIcon.png":      { icon: "/InstrumentValueIcons/camera.svg",           color: "#64d2ff" },
        "/qmlimages/LightsComponentIcon.png":      { icon: "/InstrumentValueIcons/light-bulb.svg",       color: "#ffd60a" },
        "/qmlimages/FollowComponentIcon.png":      { icon: "/InstrumentValueIcons/location-current.svg", color: "#30d158" },
        "/qmlimages/ForwardingSupportIcon.svg":    { icon: "/InstrumentValueIcons/network.svg",          color: "#8e8e93" },
        "/qmlimages/wifi.svg":                     { icon: "/InstrumentValueIcons/network.svg",          color: "#8e8e93" }
    })

    readonly property var    _tuningResources: [
        "/qmlimages/TuningComponentIcon.png",
        "/qmlimages/CameraComponentIcon.png",
        "/qmlimages/LightsComponentIcon.png",
        "/qmlimages/FollowComponentIcon.png",
        "/qmlimages/ForwardingSupportIcon.svg"
    ]

    readonly property string _fallbackTileColor: "#8e8e93"

    function _componentStyle(component) {
        return _componentStyles[String(component.iconResource)]
    }

    function componentIcon(component) {
        const style = _componentStyle(component)
        return style ? style.icon : component.iconResource
    }

    function componentColor(component) {
        const style = _componentStyle(component)
        return style ? style.color : _fallbackTileColor
    }

    function isTuning(component) {
        return _tuningResources.indexOf(String(component.iconResource)) !== -1
    }

    function matchesSearch(name) {
        return _search === "" || name.toLowerCase().indexOf(_search) !== -1
    }

    function showSummaryPanel() {
        if (mainWindow.allowViewSwitch()) {
            _showSummaryPanel()
        }
    }

    function _showSummaryPanel() {
        pageTitle = ""
        if (_fullParameterVehicleAvailable) {
            if (QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.vehicleComponents.length === 0) {
                panelLoader.loadComponent(noComponentsVehicleSummaryComponent)
            } else {
                panelLoader.loadPage("qrc:/qml/QGroundControl/VehicleSetup/VehicleSummary.qml")
            }
        } else if (QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable) {
            panelLoader.loadComponent(missingParametersVehicleSummaryComponent)
        } else {
            panelLoader.loadComponent(disconnectedVehicleSummaryComponent)
        }
        summaryButton.checked = true
    }

    function showPanel(button, qmlSource) {
        if (mainWindow.allowViewSwitch()) {
            button.checked = true
            pageTitle = button.text
            panelLoader.loadPage(qmlSource)
        }
    }

    function _checkComponentButton(name) {
        const buttons = [setupRepeater, tuningRepeater]
            .map(repeater => Array.from({ length: repeater.count }, (_, i) => repeater.itemAt(i)))
            .reduce((all, some) => all.concat(some), [])
        const button = buttons.find(b => b && b.text === name)
        if (button) {
            button.checked = true
        }
    }

    function showVehicleComponentPanel(vehicleComponent)
    {
        if (mainWindow.allowViewSwitch()) {
            var autopilotPlugin = QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin
            var prereq = autopilotPlugin.prerequisiteSetup(vehicleComponent)
            pageTitle = vehicleComponent.name
            if (prereq !== "") {
                _prerequisiteName = prereq
                _messagePanelText = qsTr("%1 has to be set up before %2.").arg(prereq).arg(vehicleComponent.name)
                panelLoader.loadComponent(messagePanelComponent)
            } else {
                panelLoader.loadPage(vehicleComponent.setupSource, vehicleComponent)
            }
            _checkComponentButton(vehicleComponent.name)
        }
    }

    function showVehicleComponentNamed(name) {
        const component = QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin
                              .vehicleComponents.find((candidate) => candidate.name === name)
        if (component) {
            showVehicleComponentPanel(component)
        }
    }

    function showParametersPanel(searchText = "") {
        if (mainWindow.allowViewSwitch()) {
            parametersButton.checked = true
            pageTitle = parametersButton.text
            panelLoader.loadPage("qrc:/qml/QGroundControl/VehicleSetup/SetupParameterEditor.qml", undefined, { initialSearchText: searchText })
        }
    }

    Component.onCompleted: _showSummaryPanel()

    Connections {
        target: QGroundControl.corePlugin
        onShowAdvancedUIChanged: {
            if(!QGroundControl.corePlugin.showAdvancedUI) {
                _showSummaryPanel()
            }
        }
    }

    Connections {
        target: QGroundControl.multiVehicleManager
        onParameterReadyVehicleAvailableChanged: {
            if (QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable || summaryButton.checked || !firmwareButton.checked) {
                summaryButton.checked = true
                _showSummaryPanel()
            }
        }
    }

    component EmptyPanel: Item {
        id: emptyPanelRoot

        property alias icon:     emptyIcon.source
        property alias headline: emptyHeadline.text
        property alias text:     emptyLabel.text
        property alias actionText: emptyAction.text
        property alias linkText:   emptyLink.text

        signal actionClicked()
        signal linkClicked()

        ColumnLayout {
            anchors.centerIn:   parent
            anchors.margins:    _defaultTextWidth * 2
            width:              Math.min(parent.width - _defaultTextWidth * 4, _defaultTextWidth * 44)
            spacing:            _defaultTextHeight / 2

            QGCColoredImage {
                id:                     emptyIcon
                Layout.alignment:       Qt.AlignHCenter
                Layout.preferredWidth:  _defaultTextHeight * 3
                Layout.preferredHeight: Layout.preferredWidth
                visible:                source.toString() !== ""
                color:                  Qt.alpha(qgcPal.text, 0.35)
                fillMode:               Image.PreserveAspectFit
                sourceSize.height:      height
            }

            QGCLabel {
                id:                     emptyHeadline
                Layout.fillWidth:       true
                Layout.topMargin:       _defaultTextHeight / 2
                visible:                text !== ""
                horizontalAlignment:    Text.AlignHCenter
                font.pointSize:         ScreenTools.mediumFontPointSize
                font.bold:              true
            }

            QGCLabel {
                id:                     emptyLabel
                Layout.fillWidth:       true
                horizontalAlignment:    Text.AlignHCenter
                wrapMode:               Text.WordWrap
                color:                  Qt.alpha(qgcPal.text, 0.6)
                onLinkActivated:        (link) => Qt.openUrlExternally(link)
            }

            QGCButton {
                id:                 emptyAction
                Layout.alignment:   Qt.AlignHCenter
                Layout.topMargin:   _defaultTextHeight / 2
                primary:            true
                visible:            text !== ""
                onClicked:          emptyPanelRoot.actionClicked()
            }

            QGCLabel {
                id:                 emptyLink
                Layout.alignment:   Qt.AlignHCenter
                Layout.topMargin:   _defaultTextHeight / 4
                visible:            text !== ""
                color:              qgcPal.colorBlue

                QGCMouseArea {
                    anchors.fill:       parent
                    anchors.margins:    -_defaultTextHeight / 3
                    onClicked:          emptyPanelRoot.linkClicked()
                }
            }
        }
    }

    Component {
        id: noComponentsVehicleSummaryComponent
        EmptyPanel {
            icon:     "/InstrumentValueIcons/checkmark-outline.svg"
            headline: qsTr("Nothing to Configure")
            text:     qsTr("%1 doesn't support setup for this vehicle type. If it is already configured, you can still fly.").arg(QGroundControl.appName)
        }
    }

    Component {
        id: disconnectedVehicleSummaryComponent
        EmptyPanel {
            icon:     "/InstrumentValueIcons/drone.svg"
            headline: qsTr("No Vehicle Connected")
            text:            qsTr("Connect a vehicle to see and change its settings.")
            actionText:      qsTr("Set Up Connection")
            onActionClicked: mainWindow.showCommLinkSettings()
            linkText:        ScreenTools.isMobile || !_corePlugin.options.showFirmwareUpgrade ? "" : qsTr("Update firmware over USB")
            onLinkClicked:   showPanel(firmwareButton, "qrc:/qml/QGroundControl/VehicleSetup/FirmwareUpgrade.qml")
        }
    }

    Component {
        id: missingParametersVehicleSummaryComponent
        EmptyPanel {
            icon:     "/InstrumentValueIcons/exclamation-outline.svg"
            headline: qsTr("Parameters Incomplete")
            text:     qsTr("The vehicle didn't return its full parameter list, so some setup options are unavailable.")
        }
    }

    Component {
        id: messagePanelComponent
        EmptyPanel {
            headline:        _prerequisiteName === "" ? "" : qsTr("%1 first").arg(_prerequisiteName)
            text:            _messagePanelText
            actionText:      _prerequisiteName === "" ? "" : qsTr("Set Up %1").arg(_prerequisiteName)
            onActionClicked: showVehicleComponentNamed(_prerequisiteName)
        }
    }

    Rectangle {
        id:                     sidebar
        anchors.left:           parent.left
        anchors.top:            parent.top
        anchors.bottom:         parent.bottom
        width:                  _sidebarWidth + _horizontalMargin * 2
        color:                  qgcPal.windowShadeDark

        Rectangle {
            anchors.right:      parent.right
            anchors.top:        parent.top
            anchors.bottom:     parent.bottom
            width:              1
            color:              Qt.alpha(qgcPal.text, 0.08)
        }
    }

    QGCTextField {
        id:                 searchField
        objectName:         "setupSearchField"
        anchors.top:        sidebar.top
        anchors.left:       sidebar.left
        anchors.right:      sidebar.right
        anchors.margins:    _horizontalMargin
        anchors.topMargin:  _verticalMargin
        leftPadding:        height * 0.8
        onAccepted: {
            if (_parametersAvailable && text.trim() !== "") {
                showParametersPanel(text.trim())
            }
        }

        background: Rectangle {
            radius:         height / 2
            color:          Qt.alpha(qgcPal.text, searchField.activeFocus ? 0.14 : 0.08)
            border.width:   searchField.activeFocus ? 1 : 0
            border.color:   qgcPal.buttonHighlight
        }

        QGCLabel {
            anchors.left:           parent.left
            anchors.leftMargin:     searchField.leftPadding
            anchors.verticalCenter: parent.verticalCenter
            visible:                searchField.text === ""
            text:                   qsTr("Search")
            color:                  Qt.alpha(qgcPal.text, 0.5)
        }

        QGCColoredImage {
            anchors.left:           parent.left
            anchors.leftMargin:     parent.height * 0.28
            anchors.verticalCenter: parent.verticalCenter
            width:                  parent.height * 0.42
            height:                 width
            source:                 "/InstrumentValueIcons/search.svg"
            color:                  Qt.alpha(qgcPal.text, 0.5)
            fillMode:               Image.PreserveAspectFit
            sourceSize.height:      height
        }
    }

    QGCFlickable {
        id:                 buttonScroll
        anchors.top:        searchField.bottom
        anchors.bottom:     sidebar.bottom
        anchors.left:       sidebar.left
        anchors.right:      sidebar.right
        anchors.margins:    _horizontalMargin
        anchors.topMargin:  _verticalMargin
        contentHeight:      buttonColumn.height + _verticalMargin
        flickableDirection: Flickable.VerticalFlick
        clip:               true

        ColumnLayout {
            id:         buttonColumn
            width:      buttonScroll.width
            spacing:    _defaultTextHeight / 4

            SettingsButton {
                id:                 summaryButton
                objectName:         "setupSummaryButton"
                Layout.fillWidth:   true
                icon.source:        "/InstrumentValueIcons/drone.svg"
                tileColor:          "#8e8e93"
                checked:            true
                text:               qsTr("Summary")
                badgeVisible:       !_setupComplete
                visible:            matchesSearch(text)
                onClicked:          showSummaryPanel()
            }

            SettingsButton {
                id:                 firmwareButton
                Layout.fillWidth:   true
                icon.source:        "/InstrumentValueIcons/cloud-upload.svg"
                tileColor:          "#64d2ff"
                visible:            !ScreenTools.isMobile && _corePlugin.options.showFirmwareUpgrade && matchesSearch(text)
                text:               qsTr("Firmware")
                badgeVisible:       _firmwareUpdateAvailable
                onClicked:          showPanel(this, "qrc:/qml/QGroundControl/VehicleSetup/FirmwareUpgrade.qml")
            }

            SettingsButton {
                Layout.fillWidth:   true
                Layout.topMargin:   _sectionGap
                icon.source:        "/InstrumentValueIcons/target.svg"
                tileColor:          "#30d158"
                visible:            (QGroundControl.multiVehicleManager.activeVehicle ? QGroundControl.multiVehicleManager.activeVehicle.flowImageIndex > 0 : false) && matchesSearch(text)
                text:               qsTr("Optical Flow")
                onClicked:          showPanel(this, "qrc:/qml/QGroundControl/VehicleSetup/OpticalFlowSensor.qml")
            }

            Repeater {
                id:     setupRepeater
                model:  _fullParameterVehicleAvailable ? QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.vehicleComponents : 0

                SettingsButton {
                    objectName:         "setupComponent" + modelData.name.replace(/\s/g, "")
                    Layout.fillWidth:   true
                    Layout.topMargin:   index === 0 ? _sectionGap : 0
                    icon.source:        componentIcon(modelData)
                    tileColor:          componentColor(modelData)
                    badgeVisible:       modelData.requiresSetup && !modelData.setupComplete
                    text:               modelData.name
                    visible:            modelData.setupSource.toString() !== "" && !isTuning(modelData) && matchesSearch(text)
                    onClicked:          showVehicleComponentPanel(modelData)
                }
            }

            Item {
                Layout.preferredHeight: _sectionGap - buttonColumn.spacing
                visible:                _hasTuning && _search === ""
            }

            Repeater {
                id:     tuningRepeater
                model:  _fullParameterVehicleAvailable ? QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.vehicleComponents : 0

                SettingsButton {
                    objectName:         "setupComponent" + modelData.name.replace(/\s/g, "")
                    Layout.fillWidth:   true
                    icon.source:        componentIcon(modelData)
                    tileColor:          componentColor(modelData)
                    badgeVisible:       modelData.requiresSetup && !modelData.setupComplete
                    text:               modelData.name
                    visible:            modelData.setupSource.toString() !== "" && isTuning(modelData) && matchesSearch(text)
                    onClicked:          showVehicleComponentPanel(modelData)
                }
            }

            SettingsButton {
                id:                 joystickButton
                Layout.fillWidth:   true
                Layout.topMargin:   _sectionGap
                icon.source:        "/InstrumentValueIcons/dial-pad.svg"
                tileColor:          "#8e8e93"
                badgeVisible:       _activeJoystick ? !(_activeJoystick.calibrated || _buttonsOnly) : false
                visible:            _fullParameterVehicleAvailable && joystickManager.joysticks.length !== 0 && matchesSearch(text)
                text:               _forcedToButtonsOnly ? qsTr("Buttons") : qsTr("Joystick")
                onClicked:          showPanel(this, "qrc:/qml/QGroundControl/VehicleSetup/JoystickConfig.qml")

                property var    _activeJoystick:        joystickManager.activeJoystick
                property bool   _buttonsOnly:           _activeJoystick ? _activeJoystick.axisCount == 0 : false
                property bool   _forcedToButtonsOnly:   !QGroundControl.corePlugin.options.allowJoystickSelection && _buttonsOnly
            }

            SettingsButton {
                id:                 parametersButton
                objectName:         "setupParametersButton"
                Layout.fillWidth:   true
                Layout.topMargin:   joystickButton.visible ? 0 : _sectionGap
                icon.source:        "/InstrumentValueIcons/list.svg"
                tileColor:          "#8e8e93"
                visible:            _parametersAvailable && matchesSearch(text)
                text:               qsTr("Parameters")
                onClicked:          showParametersPanel()
            }
        }
    }

    Loader {
        id:                     panelLoader
        anchors.leftMargin:     _defaultTextWidth * 3
        anchors.rightMargin:    _inset
        anchors.topMargin:      _inset
        anchors.bottomMargin:   _inset
        anchors.left:           sidebar.right
        anchors.right:          parent.right
        anchors.top:            parent.top
        anchors.bottom:         parent.bottom

        function loadPage(source, vehicleComponent, properties = {}) {
            panelLoader.source = ""
            panelLoader.vehicleComponent = vehicleComponent
            panelLoader.setSource(source, properties)
        }

        function loadComponent(sourceComponent, vehicleComponent) {
            panelLoader.sourceComponent = undefined
            panelLoader.vehicleComponent = vehicleComponent
            panelLoader.sourceComponent = sourceComponent
        }

        property var vehicleComponent
    }
}
