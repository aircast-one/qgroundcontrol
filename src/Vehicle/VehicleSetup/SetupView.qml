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

Rectangle {
    id:         setupView
    objectName: "vehicleSetupView"
    color:      "transparent"
    z:          QGroundControl.zOrderTopMost

    DeadMouseArea {
        anchors.fill: parent
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    readonly property real      _defaultTextHeight: ScreenTools.defaultFontPixelHeight
    readonly property real      _defaultTextWidth:  ScreenTools.defaultFontPixelWidth
    readonly property real      _horizontalMargin:  _defaultTextWidth / 2
    readonly property real      _verticalMargin:    _defaultTextHeight / 2
    readonly property real      _inset:             Math.round(_defaultTextHeight * 0.75)
    readonly property real      _sidebarRadius:     mainWindow.panelRadius - _inset
    readonly property real      _sidebarWidth:      _defaultTextWidth * 26
    readonly property real      _sectionGap:        _defaultTextHeight * 0.75
    readonly property string    _armedVehicleText:  qsTr("This operation cannot be performed while the vehicle is armed.")

    property bool   _vehicleArmed:                  QGroundControl.multiVehicleManager.activeVehicle ? QGroundControl.multiVehicleManager.activeVehicle.armed : false
    property string _messagePanelText:              qsTr("missing message panel text")
    property bool   _fullParameterVehicleAvailable: QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable && !QGroundControl.multiVehicleManager.activeVehicle.parameterManager.missingParameters
    property var    _corePlugin:                    QGroundControl.corePlugin
    property string _pageTitle
    property bool   _setupComplete:                 _fullParameterVehicleAvailable ? QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.setupComplete : true
    property bool   _parametersAvailable:           QGroundControl.multiVehicleManager.parameterReadyVehicleAvailable &&
                                                    !QGroundControl.multiVehicleManager.activeVehicle.usingHighLatencyLink &&
                                                    _corePlugin.showAdvancedUI

    readonly property string _search:       searchField.text.trim().toLowerCase()
    readonly property bool   _hasTuning:    _fullParameterVehicleAvailable &&
                                            QGroundControl.multiVehicleManager.activeVehicle.autopilotPlugin.vehicleComponents
                                                .some(component => isTuning(component.name) && component.setupSource.toString() !== "")
    readonly property var    _tuningNames:  ["tuning", "camera", "lights", "follow me", "remote support"]
    readonly property var    _tileColors:   ({
        "summary":      "#8e8e93",
        "firmware":     "#64d2ff",
        "airframe":     "#ff9f0a",
        "frame":        "#ff9f0a",
        "sensors":      "#30d158",
        "optical flow": "#30d158",
        "radio":        "#0a84ff",
        "flight modes": "#5e5ce6",
        "power":        "#ff375f",
        "motors":       "#bf5af2",
        "actuators":    "#bf5af2",
        "safety":       "#ff453a",
        "tuning":       "#ac8e68",
        "camera":       "#64d2ff",
        "lights":       "#ffd60a",
        "follow":       "#30d158",
        "joystick":     "#8e8e93",
        "parameters":   "#8e8e93"
    })

    function tileColorFor(name) {
        return _tileColors[name.toLowerCase()] || "#8e8e93"
    }

    function isTuning(name) {
        return _tuningNames.indexOf(name.toLowerCase()) !== -1
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
        _pageTitle = ""
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
            _pageTitle = button.text
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
            _pageTitle = vehicleComponent.name
            if (prereq !== "") {
                _messagePanelText = qsTr("%1 setup must be completed prior to %2 setup.").arg(prereq).arg(vehicleComponent.name)
                panelLoader.loadComponent(messagePanelComponent)
            } else {
                panelLoader.loadPage(vehicleComponent.setupSource, vehicleComponent)
            }
            _checkComponentButton(vehicleComponent.name)
        }
    }

    function showParametersPanel(searchText = "") {
        if (mainWindow.allowViewSwitch()) {
            parametersButton.checked = true
            _pageTitle = parametersButton.text
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
        property alias text: emptyLabel.text

        QGCLabel {
            id:                     emptyLabel
            anchors.margins:        _defaultTextWidth * 2
            anchors.fill:           parent
            verticalAlignment:      Text.AlignVCenter
            horizontalAlignment:    Text.AlignHCenter
            wrapMode:               Text.WordWrap
            font.pointSize:         ScreenTools.mediumFontPointSize
            color:                  Qt.alpha(qgcPal.text, 0.6)
            onLinkActivated:        (link) => Qt.openUrlExternally(link)
        }
    }

    Component {
        id: noComponentsVehicleSummaryComponent
        EmptyPanel {
            text: qsTr("%1 does not currently support setup of your vehicle type. ").arg(QGroundControl.appName) +
                  "If your vehicle is already configured you can still Fly."
        }
    }

    Component {
        id: disconnectedVehicleSummaryComponent
        EmptyPanel {
            text: qsTr("Vehicle settings and info will display after connecting your vehicle.") +
                  (ScreenTools.isMobile || !_corePlugin.options.showFirmwareUpgrade ? "" : " Click Firmware on the left to upgrade your vehicle.")
        }
    }

    Component {
        id: missingParametersVehicleSummaryComponent
        EmptyPanel {
            text: qsTr("You are currently connected to a vehicle but it did not return the full parameter list. ") +
                  qsTr("As a result, the full set of vehicle setup options are not available.")
        }
    }

    Component {
        id: messagePanelComponent
        EmptyPanel {
            text: _messagePanelText
        }
    }

    OverlayGlass {
        id:                     sidebarSlab
        anchors.left:           parent.left
        anchors.top:            parent.top
        anchors.bottom:         parent.bottom
        anchors.margins:        _inset
        width:                  _sidebarWidth + _horizontalMargin * 2
        radius:                 _sidebarRadius
        frosted:                mainWindow.flyViewBackdropVisible
        tint:                   QGroundControl.globalPalette.windowShade
        material:               OverlayGlass.Panel
    }

    QGCTextField {
        id:                 searchField
        objectName:         "setupSearchField"
        anchors.top:        sidebarSlab.top
        anchors.left:       sidebarSlab.left
        anchors.right:      sidebarSlab.right
        anchors.margins:    _horizontalMargin
        anchors.topMargin:  _verticalMargin
        placeholderText:    qsTr("Search")
        onAccepted: {
            if (_parametersAvailable && text.trim() !== "") {
                showParametersPanel(text.trim())
            }
        }
    }

    QGCFlickable {
        id:                 buttonScroll
        anchors.top:        searchField.bottom
        anchors.bottom:     sidebarSlab.bottom
        anchors.left:       sidebarSlab.left
        anchors.right:      sidebarSlab.right
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
                icon.source:        "/qmlimages/VehicleSummaryIcon.png"
                tileColor:          tileColorFor(text)
                checked:            true
                text:               qsTr("Summary")
                badgeVisible:       !_setupComplete
                visible:            matchesSearch(text)
                onClicked:          showSummaryPanel()
            }

            SettingsButton {
                id:                 firmwareButton
                Layout.fillWidth:   true
                icon.source:        "/qmlimages/FirmwareUpgradeIcon.png"
                tileColor:          tileColorFor(text)
                visible:            !ScreenTools.isMobile && _corePlugin.options.showFirmwareUpgrade && matchesSearch(text)
                text:               qsTr("Firmware")
                onClicked:          showPanel(this, "qrc:/qml/QGroundControl/VehicleSetup/FirmwareUpgrade.qml")
            }

            SettingsButton {
                Layout.fillWidth:   true
                Layout.topMargin:   _sectionGap
                icon.source:        "/qmlimages/SensorsComponentIcon.png"
                tileColor:          tileColorFor(text)
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
                    icon.source:        modelData.iconResource
                    tileColor:          tileColorFor(text)
                    badgeVisible:       modelData.requiresSetup && !modelData.setupComplete
                    text:               modelData.name
                    visible:            modelData.setupSource.toString() !== "" && !isTuning(modelData.name) && matchesSearch(text)
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
                    icon.source:        modelData.iconResource
                    tileColor:          tileColorFor(text)
                    badgeVisible:       modelData.requiresSetup && !modelData.setupComplete
                    text:               modelData.name
                    visible:            modelData.setupSource.toString() !== "" && isTuning(modelData.name) && matchesSearch(text)
                    onClicked:          showVehicleComponentPanel(modelData)
                }
            }

            SettingsButton {
                id:                 joystickButton
                Layout.fillWidth:   true
                Layout.topMargin:   _sectionGap
                icon.source:        "/qmlimages/Joystick.png"
                tileColor:          tileColorFor("joystick")
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
                tileColor:          tileColorFor(text)
                visible:            _parametersAvailable && matchesSearch(text)
                text:               qsTr("Parameters")
                onClicked:          showParametersPanel()
            }
        }
    }

    QGCLabel {
        id:                     pageTitle
        anchors.leftMargin:     _defaultTextWidth * 3
        anchors.topMargin:      _inset
        anchors.left:           sidebarSlab.right
        anchors.top:            parent.top
        text:                   _pageTitle
        font.pointSize:         ScreenTools.largeFontPointSize
        font.bold:              true
        visible:                text !== ""
    }

    Loader {
        id:                     panelLoader
        anchors.leftMargin:     _defaultTextWidth * 3
        anchors.rightMargin:    _inset
        anchors.topMargin:      pageTitle.visible ? _verticalMargin : _inset
        anchors.bottomMargin:   _inset
        anchors.left:           sidebarSlab.right
        anchors.right:          parent.right
        anchors.top:            pageTitle.visible ? pageTitle.bottom : parent.top
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
