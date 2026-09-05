/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.FactSystem

RowLayout {
    id:         control
    spacing:    _pillPadding + ScreenTools.defaultFontPixelWidth

    readonly property real _pillPadding: ScreenTools.defaultFontPixelWidth * 1.6

    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property var    _vehicleInAir:      _activeVehicle ? _activeVehicle.flying || _activeVehicle.landing : false
    property bool   _vtolInFWDFlight:   _activeVehicle ? _activeVehicle.vtolInFwdFlight : false
    property bool   _armed:             _activeVehicle ? _activeVehicle.armed : false
    property real   _margins:           ScreenTools.defaultFontPixelWidth
    property real   _spacing:           ScreenTools.defaultFontPixelWidth / 2
    property bool   _healthAndArmingChecksSupported: _activeVehicle ? _activeVehicle.healthAndArmingCheckReport.supported : false

    function dropMainStatusIndicator() {
        let overallStatusComponent = _activeVehicle ? overallStatusIndicatorPage : overallStatusOfflineIndicatorPage
        mainWindow.showIndicatorDrawer(overallStatusComponent, control)
    }

    QGCLabel {
        id:                     mainStatusLabel
        Layout.fillHeight:      true
        Layout.preferredWidth:  statusPill.width
        leftPadding:            control._pillPadding
        verticalAlignment:      Text.AlignVCenter
        text:                   mainStatusText()
        color:                  qgcPal.toolbarText
        font.pointSize:         ScreenTools.largeFontPointSize

        readonly property real _iconSlot: vehicleMessagesIcon.visible ? vehicleMessagesIcon.width + ScreenTools.defaultFontPixelWidth : 0

        OverlayCapsule {
            id:                     statusPill
            objectName:             "mainStatusPill"
            z:                      -1
            anchors.verticalCenter: parent.verticalCenter
            anchors.left:           parent.left
            width:                  parent.contentWidth + parent._iconSlot + control._pillPadding * 2
            height:                 parent.contentHeight + ScreenTools.defaultFontPixelHeight
        }

        property string _commLostText:      qsTr("Comms Lost")
        property string _readyToFlyText:    qsTr("Ready To Fly")
        property string _notReadyToFlyText: qsTr("Not Ready")
        property string _disconnectedText:  qsTr("Not connected")
        property string _armedText:         qsTr("Armed")
        property string _flyingText:        qsTr("Flying")
        property string _landingText:       qsTr("Landing")

        function mainStatusText() {
            var statusText
            if (_activeVehicle) {
                if (_communicationLost) {
                    _mainStatusBGColor = "red"
                    return mainStatusLabel._commLostText
                }
                if (_activeVehicle.armed) {
                    _mainStatusBGColor = "green"

                    if (_healthAndArmingChecksSupported) {
                        if (_activeVehicle.healthAndArmingCheckReport.canArm) {
                            if (_activeVehicle.healthAndArmingCheckReport.hasWarningsOrErrors) {
                                _mainStatusBGColor = "yellow"
                            }
                        } else {
                            _mainStatusBGColor = "red"
                        }
                    }

                    if (_activeVehicle.flying) {
                        return mainStatusLabel._flyingText
                    } else if (_activeVehicle.landing) {
                        return mainStatusLabel._landingText
                    } else {
                        return mainStatusLabel._armedText
                    }
                } else {
                    if (_healthAndArmingChecksSupported) {
                        if (_activeVehicle.healthAndArmingCheckReport.canArm) {
                            if (_activeVehicle.healthAndArmingCheckReport.hasWarningsOrErrors) {
                                _mainStatusBGColor = "yellow"
                            } else {
                                _mainStatusBGColor = "green"
                            }
                            return mainStatusLabel._readyToFlyText
                        } else {
                            _mainStatusBGColor = "red"
                            return mainStatusLabel._notReadyToFlyText
                        }
                    } else if (_activeVehicle.readyToFlyAvailable) {
                        if (_activeVehicle.readyToFly) {
                            _mainStatusBGColor = "green"
                            return mainStatusLabel._readyToFlyText
                        } else {
                            _mainStatusBGColor = "yellow"
                            return mainStatusLabel._notReadyToFlyText
                        }
                    } else {
                        if (_activeVehicle.allSensorsHealthy && _activeVehicle.autopilotPlugin.setupComplete) {
                            _mainStatusBGColor = "green"
                            return mainStatusLabel._readyToFlyText
                        } else {
                            _mainStatusBGColor = "yellow"
                            return mainStatusLabel._notReadyToFlyText
                        }
                    }
                }
            } else {
                _mainStatusBGColor = qgcPal.brandingPurple
                return mainStatusLabel._disconnectedText
            }
        }

        QGCColoredImage {
            id:                     vehicleMessagesIcon
            anchors.verticalCenter: parent.verticalCenter
            anchors.right:          parent.right
            anchors.rightMargin:    control._pillPadding
            width:                  ScreenTools.defaultFontPixelWidth * 2
            height:                 width
            source:                 "/res/VehicleMessages.png"
            color:                  getIconColor()
            sourceSize.width:       width
            fillMode:               Image.PreserveAspectFit
            visible:                _activeVehicle && _activeVehicle.messageCount > 0

            function getIconColor() {
                let iconColor = qgcPal.toolbarText
                if (_activeVehicle) {
                    if (_activeVehicle.messageTypeWarning) {
                        iconColor = qgcPal.colorOrange
                    } else if (_activeVehicle.messageTypeError) {
                        iconColor = qgcPal.colorRed
                    }
                }
                return iconColor
            }
        }

        QGCMouseArea {
            anchors.fill:   parent
            onClicked:      dropMainStatusIndicator()
        }
    }

    QGCLabel {
        id:                 vtolModeLabel
        Layout.fillHeight:  true
        verticalAlignment:  Text.AlignVCenter
        color:              qgcPal.toolbarText
        text:               _vtolInFWDFlight ? qsTr("FW(vtol)") : qsTr("MR(vtol)")
        font.pointSize:     _vehicleInAir ? ScreenTools.largeFontPointSize : ScreenTools.defaultFontPointSize
        visible:            _activeVehicle && _activeVehicle.vtol

        QGCMouseArea {
            anchors.fill: parent
            onClicked: {
                if (_vehicleInAir) {
                    mainWindow.showIndicatorDrawer(vtolTransitionIndicatorPage)
                }
            }
        }
    }

    Component {
        id: overallStatusOfflineIndicatorPage

        MainStatusIndicatorOfflinePage { }
    }

    Component {
        id: overallStatusIndicatorPage

        ToolIndicatorPage {
            showExpand:         _activeVehicle.mainStatusIndicatorContentItem ? true : false
            waitForParameters:  _activeVehicle.mainStatusIndicatorContentItem ? true : false
            contentComponent:   mainStatusContentComponent
            expandedComponent:  mainStatusExpandedComponent
        }
    }

    Component {
        id: mainStatusContentComponent

        ColumnLayout {
            id:                     mainLayout
            spacing:                ScreenTools.defaultFontPixelHeight * 0.75
            Layout.minimumWidth:    ScreenTools.defaultFontPixelWidth * 36

            readonly property var  _sensorNames:   _healthAndArmingChecksSupported ? [] : _activeVehicle.sysStatusSensorInfo.sensorNames
            readonly property var  _sensorStatus:  _healthAndArmingChecksSupported ? [] : _activeVehicle.sysStatusSensorInfo.sensorStatus
            readonly property var  _sensorHealthy: _healthAndArmingChecksSupported ? [] : _activeVehicle.sysStatusSensorInfo.sensorHealthy
            readonly property var  _sensorIssues:  _sensorNames.filter((_, i) => !_sensorHealthy[i])
            readonly property int  _checkIssues:   _healthAndArmingChecksSupported ? _activeVehicle.healthAndArmingCheckReport.problemsForCurrentMode.count : 0
            readonly property bool _canArm:        _armed || !_healthAndArmingChecksSupported || _activeVehicle.healthAndArmingCheckReport.canArm
            property bool          _showForceArm:  false

            readonly property string _summaryDetail: {
                if (_armed) {
                    return _vehicleInAir ? qsTr("Motors are armed and the vehicle is in the air.") : qsTr("Motors are armed. Keep clear of the propellers.")
                }
                if (_checkIssues > 0) {
                    return qsTr("%n check(s) need attention before arming.", "", _checkIssues)
                }
                if (_sensorIssues.length > 0) {
                    return qsTr("%1 unavailable. Position modes and Return to Launch may not work.").arg(_sensorIssues.join(", "))
                }
                return qsTr("All checks passed.")
            }

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelHeight / 6

                QGCLabel {
                    text:           mainStatusLabel.mainStatusText()
                    font.pointSize: ScreenTools.mediumFontPointSize
                    font.bold:      true
                }

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               mainLayout._summaryDetail
                    color:              qgcPal.colorGrey
                    wrapMode:           Text.WordWrap
                    font.pointSize:     ScreenTools.smallFontPointSize
                }
            }

            SettingsGroupLayout {
                popoverStyle: true
                heading:            qsTr("Vehicle Messages")
                visible:            !vehicleMessageList.noMessages

                VehicleMessageList { 
                    id: vehicleMessageList
                }
            }

            SettingsGroupLayout {
                popoverStyle:       true
                Layout.fillWidth:   true
                heading:            qsTr("Sensors")
                visible:            !_healthAndArmingChecksSupported

                Repeater {
                    model: mainLayout._sensorNames

                    RowLayout {
                        Layout.fillWidth:   true
                        spacing:            ScreenTools.defaultFontPixelWidth * 2

                        required property int    index
                        required property string modelData

                        QGCLabel { Layout.fillWidth: true; text: modelData }

                        QGCLabel {
                            text:   mainLayout._sensorStatus[index]
                            color:  mainLayout._sensorHealthy[index] ? qgcPal.colorGrey : qgcPal.colorOrange
                        }
                    }
                }
            }

            SettingsGroupLayout {
                popoverStyle: true
                heading:            qsTr("Overall Status")
                visible:            _healthAndArmingChecksSupported && _activeVehicle.healthAndArmingCheckReport.problemsForCurrentMode.count > 0

                Repeater {
                    model:      _activeVehicle ? _activeVehicle.healthAndArmingCheckReport.problemsForCurrentMode : null
                    delegate:   listdelegate
                }
            }

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelHeight / 3

                SliderSwitch {
                    Layout.fillWidth:   true
                    confirmText:        _armed ? qsTr("Slide to Disarm") : qsTr("Slide to Arm")
                    enabled:            mainLayout._canArm
                    opacity:            enabled ? 1 : 0.4
                    onAccept: {
                        _armed ? mainWindow.disarmVehicleRequest() : mainWindow.armVehicleRequest()
                        mainWindow.closeIndicatorDrawer()
                    }
                }

                QGCLabel {
                    Layout.alignment:   Qt.AlignHCenter
                    text:               qsTr("Force Arm…")
                    color:              qgcPal.colorOrange
                    font.pointSize:     ScreenTools.smallFontPointSize
                    visible:            !_armed && !mainLayout._canArm && !mainLayout._showForceArm

                    QGCMouseArea {
                        anchors.fill:   parent
                        onClicked:      mainLayout._showForceArm = true
                    }
                }

                SliderSwitch {
                    Layout.fillWidth:   true
                    confirmText:        qsTr("Slide to Force Arm")
                    visible:            !_armed && mainLayout._showForceArm
                    onAccept: {
                        mainWindow.forceArmVehicleRequest()
                        mainWindow.closeIndicatorDrawer()
                    }
                }
            }

            OverlayMenuSeparator { Layout.fillWidth: true }

            OverlayMenuItem {
                objectName:         "vehicleSetupItem"
                Layout.fillWidth:   true
                icon:               "/InstrumentValueIcons/wrench.svg"
                text:               qsTr("Vehicle Setup")
                onClicked: {
                    if (mainWindow.allowViewSwitch()) {
                        mainWindow.closeIndicatorDrawer()
                        mainWindow.showVehicleConfig()
                    }
                }
            }

            FactPanelController {
                id: controller
            }

            Component {
                id: listdelegate

                Column {
                    Row {
                        spacing: ScreenTools.defaultFontPixelHeight

                        QGCLabel {
                            id:           message
                            text:         object.message
                            textFormat:   TextEdit.RichText
                            color:        object.severity == 'error' ? qgcPal.colorRed : object.severity == 'warning' ? qgcPal.colorOrange : qgcPal.text
                            MouseArea {
                                anchors.fill: parent
                                onClicked: {
                                    if (object.description != "")
                                        object.expanded = !object.expanded
                                }
                            }
                        }

                        QGCColoredImage {
                            id:                     arrowDownIndicator
                            anchors.verticalCenter: parent.verticalCenter
                            height:                 1.5 * ScreenTools.defaultFontPixelWidth
                            width:                  height
                            source:                 "/qmlimages/arrow-down.png"
                            color:                  qgcPal.text
                            visible:                object.description != ""
                            MouseArea {
                                anchors.fill:       parent
                                onClicked:          object.expanded = !object.expanded
                            }
                        }
                    }

                    QGCLabel {
                        id:                 description
                        text:               object.description
                        textFormat:         TextEdit.RichText
                        clip:               true
                        visible:            object.expanded
                        
                        property var fact:  null

                        onLinkActivated: (link) => {
                            if (link.startsWith('param://')) {
                                var paramName = link.substr(8);
                                fact = controller.getParameterFact(-1, paramName, true)
                                if (fact != null) {
                                    paramEditorDialogComponent.createObject(mainWindow).open()
                                }
                            } else {
                                Qt.openUrlExternally(link);
                            }
                        }

                        Component {
                            id: paramEditorDialogComponent

                            ParameterEditorDialog {
                                title:          qsTr("Edit Parameter")
                                fact:           description.fact
                                destroyOnClose: true
                            }
                        }
                    }
                }
            }
        }
    }

    Component {
        id: mainStatusExpandedComponent

        ColumnLayout {
            Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 60
            spacing:                margins / 2

            property real margins: ScreenTools.defaultFontPixelHeight

            Loader {
                source: _activeVehicle.mainStatusIndicatorContentItem
            }

            SettingsGroupLayout {
                popoverStyle: true
                Layout.fillWidth:   true
                visible:            QGroundControl.corePlugin.showAdvancedUI

                GridLayout {
                    columns:            2
                    rowSpacing:         ScreenTools.defaultFontPixelHeight / 2
                    columnSpacing:      ScreenTools.defaultFontPixelWidth *2
                    Layout.fillWidth:   true

                    QGCLabel { Layout.fillWidth: true; text: qsTr("Vehicle Parameters") }
                    QGCButton {
                        text: qsTr("Configure")
                        onClicked: {                            
                            mainWindow.showVehicleConfigParametersPage()
                            mainWindow.closeIndicatorDrawer()
                        }
                    }

                    QGCLabel { Layout.fillWidth: true; text: qsTr("Vehicle Configuration") }
                    QGCButton {
                        text: qsTr("Configure")
                        onClicked: {                            
                            mainWindow.showVehicleConfig()
                            mainWindow.closeIndicatorDrawer()
                        }
                    }
                }
            }
        }
    }

    Component {
        id: vtolTransitionIndicatorPage

        ToolIndicatorPage {
            contentComponent: Component {
                QGCButton {
                    text: _vtolInFWDFlight ? qsTr("Transition to Multi-Rotor") : qsTr("Transition to Fixed Wing")

                    onClicked: {
                        if (_vtolInFWDFlight) {
                            mainWindow.vtolTransitionToMRFlightRequest()
                        } else {
                            mainWindow.vtolTransitionToFwdFlightRequest()
                        }
                        mainWindow.closeIndicatorDrawer()
                    }
                }
            }
        }
    }
}

