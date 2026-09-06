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

    readonly property var _sysStatusSensors: (_activeVehicle && !_healthAndArmingChecksSupported) ? _activeVehicle.sysStatusSensorInfo : null
    readonly property var _healthReport:     (_activeVehicle && _healthAndArmingChecksSupported)  ? _activeVehicle.healthAndArmingCheckReport : null
    readonly property var _sensorNames:      _sysStatusSensors ? _sysStatusSensors.sensorNames    : []
    readonly property var _sensorStatus:     _sysStatusSensors ? _sysStatusSensors.sensorStatus   : []
    readonly property var _sensorHealthy:    _sysStatusSensors ? _sysStatusSensors.sensorHealthy  : []
    readonly property var _sensorEnabled:    _sysStatusSensors ? _sysStatusSensors.sensorEnabled  : []

    readonly property real _messageIconFloor: ScreenTools.defaultFontPixelWidth * 20

    readonly property real minimumWidth: mainStatusLabel.Layout.minimumWidth
    readonly property real legibleWidth: _messageIconFloor

    readonly property color _statusColor: statusSummary.fault   ? qgcPal.colorRed
                                        : statusSummary.caution ? qgcPal.colorOrange
                                                                : qgcPal.text

    VehicleStatusSummary {
        id:                     statusSummary
        sensorNames:            control._sensorNames
        sensorHealthy:          control._sensorHealthy
        sensorEnabled:          control._sensorEnabled
        healthChecksSupported:  control._healthAndArmingChecksSupported
        canArm:                 control._healthReport ? control._healthReport.canArm : true
        hasWarningsOrErrors:    control._healthReport ? control._healthReport.hasWarningsOrErrors : false
    }

    readonly property bool _noUserLinks: {
        const configs = QGroundControl.linkManager.linkConfigurations
        return !Array.from({ length: configs.count }, (_, i) => configs.get(i))
                     .some((config) => !config.dynamic && !config.isAutoConnect)
    }

    function dropMainStatusIndicator() {
        let overallStatusComponent = _activeVehicle ? overallStatusIndicatorPage : overallStatusOfflineIndicatorPage
        mainWindow.showIndicatorDrawer(overallStatusComponent, control)
    }

    QGCLabel {
        id:                     mainStatusLabel
        Layout.fillHeight:      true
        Layout.fillWidth:       true
        Layout.preferredWidth:  Math.ceil(statusMetrics.advanceWidth) + leftPadding + _iconSlot + control._pillPadding
        Layout.maximumWidth:    Layout.preferredWidth
        Layout.minimumWidth:    leftPadding + control._pillPadding
        leftPadding:            control._pillPadding
        rightPadding:           (chevron.visible ? _iconSlot : _iconSlot - _chevronSlot) + control._pillPadding
        verticalAlignment:      Text.AlignVCenter
        elide:                  Text.ElideRight
        text:                   mainStatusText()
        color:                  statusSummary.nominal ? qgcPal.toolbarText : control._statusColor
        font.bold:              true

        TextMetrics {
            id:   statusMetrics
            font: mainStatusLabel.font
            text: mainStatusLabel.text
        }

        readonly property real _chevronSlot: chevron.width + ScreenTools.defaultFontPixelWidth
        readonly property real _iconSlot:    (vehicleMessagesIcon.visible ? vehicleMessagesIcon.width + ScreenTools.defaultFontPixelWidth : 0) + _chevronSlot

        OverlayCapsule {
            id:                     statusPill
            objectName:             "mainStatusPill"
            z:                      -1
            anchors.verticalCenter: parent.verticalCenter
            anchors.left:           parent.left
            width:                  parent.width
            height:                 parent.height
        }

        property string _commLostText:      qsTr("Comms Lost")
        property string _readyToFlyText:    statusSummary.nominal ? qsTr("Ready to Fly") : qsTr("Not Fully Ready")
        property string _notReadyToFlyText: qsTr("Not Ready")
        property string _disconnectedText:  control._noUserLinks ? qsTr("Connect a Vehicle") : qsTr("Not Connected")
        property string _connectingText:    qsTr("Connecting…")
        property string _failedText:        qsTr("Can't Connect")
        property string _armedText:         qsTr("Armed")
        property string _flyingText:        qsTr("Flying")
        property string _landingText:       qsTr("Landing")

        function mainStatusText() {
            if (!_activeVehicle) {
                return QGroundControl.linkManager.connectingLinkName !== "" ? mainStatusLabel._connectingText
                     : QGroundControl.linkManager.failedLinkName !== ""     ? mainStatusLabel._failedText
                                                                            : mainStatusLabel._disconnectedText
            }
            if (_communicationLost) {
                return mainStatusLabel._commLostText
            }
            if (_activeVehicle.armed) {
                return _activeVehicle.flying  ? mainStatusLabel._flyingText
                     : _activeVehicle.landing ? mainStatusLabel._landingText
                                              : mainStatusLabel._armedText
            }
            if (_healthAndArmingChecksSupported) {
                return _activeVehicle.healthAndArmingCheckReport.canArm ? mainStatusLabel._readyToFlyText
                                                                        : mainStatusLabel._notReadyToFlyText
            }
            if (_activeVehicle.readyToFlyAvailable) {
                return _activeVehicle.readyToFly ? mainStatusLabel._readyToFlyText : mainStatusLabel._notReadyToFlyText
            }
            return _activeVehicle.allSensorsHealthy && _activeVehicle.autopilotPlugin.setupComplete
                        ? mainStatusLabel._readyToFlyText : mainStatusLabel._notReadyToFlyText
        }

        QGCColoredImage {
            id:                     chevron
            anchors.verticalCenter: parent.verticalCenter
            anchors.right:          parent.right
            anchors.rightMargin:    control._pillPadding
            width:                  ScreenTools.defaultFontPixelHeight * 0.7
            height:                 width
            source:                 "/InstrumentValueIcons/cheveron-down.svg"
            color:                  Qt.alpha(qgcPal.toolbarText, 0.6)
            visible:                mainStatusLabel.width >= mainStatusLabel.Layout.preferredWidth - 0.5
            sourceSize.height:      height
            fillMode:               Image.PreserveAspectFit
            mipmap:                 true
        }

        QGCColoredImage {
            id:                     vehicleMessagesIcon
            anchors.verticalCenter: parent.verticalCenter
            anchors.right:          chevron.left
            anchors.rightMargin:    ScreenTools.defaultFontPixelWidth
            width:                  ScreenTools.defaultFontPixelWidth * 2
            height:                 width
            source:                 "/res/VehicleMessages.png"
            color:                  getIconColor()
            sourceSize.width:       width
            fillMode:               Image.PreserveAspectFit
            visible:                _activeVehicle && _activeVehicle.messageCount > 0 &&
                                        mainStatusLabel.width > control._messageIconFloor

            function getIconColor() {
                if (!_activeVehicle) {
                    return qgcPal.toolbarText
                }
                return _activeVehicle.messageTypeWarning ? qgcPal.colorOrange
                     : _activeVehicle.messageTypeError   ? qgcPal.colorRed
                                                         : qgcPal.toolbarText
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
            spacing:                ScreenTools.defaultFontPixelHeight * 0.6
            Layout.minimumWidth:    ScreenTools.defaultFontPixelWidth * 36

            readonly property int  _checkIssues:    _healthAndArmingChecksSupported ? _activeVehicle.healthAndArmingCheckReport.problemsForCurrentMode.count : 0
            readonly property bool _canArm:         _armed || !_healthAndArmingChecksSupported || _activeVehicle.healthAndArmingCheckReport.canArm
            property bool          _showForceArm:   false
            property bool          _showAllSensors: false
            property bool          _showMessages:   false

            readonly property color _badgeColor: statusSummary.fault   ? qgcPal.colorRed
                                               : statusSummary.caution ? qgcPal.colorOrange
                                                                       : qgcPal.colorGreen

            function openVehicleSetup() {
                if (mainWindow.allowViewSwitch()) {
                    mainWindow.closeIndicatorDrawer()
                    mainWindow.showVehicleConfig()
                }
            }

            readonly property string _summaryDetail: {
                if (_armed) {
                    return _vehicleInAir ? qsTr("Motors are armed and the vehicle is in the air.") : qsTr("Motors are armed. Keep clear of the propellers.")
                }
                if (_checkIssues > 0) {
                    return qsTr("%n check(s) need attention before arming.", "", _checkIssues)
                }
                if (statusSummary.faults.length > 0) {
                    return qsTr("%1 not working. Position modes and Return to Launch may not work.").arg(statusSummary.faultList)
                }
                if (statusSummary.disabled.length > 0) {
                    return qsTr("%1 turned off. Everything else reports normal.").arg(statusSummary.disabledList)
                }
                return qsTr("All checks passed.")
            }

            RowLayout {
                Layout.fillWidth:   true
                Layout.leftMargin:  ScreenTools.defaultFontPixelWidth * 1.5
                Layout.rightMargin: ScreenTools.defaultFontPixelWidth * 1.5
                spacing:            ScreenTools.defaultFontPixelWidth

                QGCColoredImage {
                    Layout.alignment:       Qt.AlignTop
                    Layout.topMargin:       ScreenTools.defaultFontPixelHeight * 0.2
                    Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 1.15
                    Layout.preferredWidth:  Layout.preferredHeight
                    source:                 statusSummary.nominal ? "/InstrumentValueIcons/checkmark-outline.svg"
                                                                  : "/InstrumentValueIcons/exclamation-outline.svg"
                    color:                  mainLayout._badgeColor
                    sourceSize.height:      height
                    fillMode:               Image.PreserveAspectFit
                    mipmap:                 true
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
                        color:              Qt.alpha(qgcPal.text, 0.6)
                        wrapMode:           Text.WordWrap
                        font.pointSize:     ScreenTools.smallFontPointSize
                    }
                }
            }

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelHeight / 3

                SliderSwitch {
                    objectName:         "armSlider"
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
                    Layout.fillWidth:       true
                    Layout.leftMargin:      ScreenTools.defaultFontPixelWidth * 1.5
                    Layout.rightMargin:     ScreenTools.defaultFontPixelWidth * 1.5
                    wrapMode:               Text.WordWrap
                    text:                   qsTr("Arming may be refused.")
                    color:                  Qt.alpha(qgcPal.text, 0.6)
                    font.pointSize:         ScreenTools.smallFontPointSize
                    visible:                !_armed && !statusSummary.nominal && mainLayout._canArm && !mainLayout._showForceArm
                }

                QGCLabel {
                    Layout.fillWidth:   true
                    Layout.leftMargin:  ScreenTools.defaultFontPixelWidth * 1.5
                    text:               qsTr("Force Arm…")
                    color:              qgcPal.colorBlue
                    font.pointSize:     ScreenTools.smallFontPointSize
                    visible:            !_armed && !mainLayout._showForceArm && (!mainLayout._canArm || statusSummary.fault)

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

            SettingsGroupLayout {
                popoverStyle:       true
                cardStyle:          true
                insetContent:       false
                showDividers:       false
                Layout.fillWidth:   true
                heading:            qsTr("Messages")
                headingAction:      vehicleMessageList.visible ? qsTr("Clear") : ""
                visible:            !vehicleMessageList.noMessages

                onHeadingActionClicked: {
                    _activeVehicle.clearMessages()
                    vehicleMessageList.clear()
                }

                OverlayDisclosureRow {
                    objectName: "messageDisclosure"
                    text:       mainLayout._showMessages ? qsTr("Hide Messages") : qsTr("Show Messages")
                    onClicked:  mainLayout._showMessages = !mainLayout._showMessages
                }

                VehicleMessageList {
                    id:      vehicleMessageList
                    visible: mainLayout._showMessages
                }
            }

            SettingsGroupLayout {
                popoverStyle:       true
                cardStyle:          true
                insetContent:       false
                contentSpacing:     0
                Layout.fillWidth:   true
                heading:            qsTr("Sensors")
                visible:            !_healthAndArmingChecksSupported && control._sensorNames.length > 0

                Repeater {
                    model: control._sensorNames

                    Rectangle {
                        Layout.fillWidth:       true
                        Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 1.9
                        radius:                 ScreenTools.defaultFontPixelHeight / 3
                        color:                  sensorMouseArea.containsMouse ? Qt.alpha(qgcPal.text, 0.08) : "transparent"
                        visible:                mainLayout._showAllSensors || !control._sensorHealthy[index]

                        required property int    index
                        required property string modelData

                        readonly property bool _fault: control._sensorEnabled[index] && !control._sensorHealthy[index]

                        RowLayout {
                            id:                     sensorRow
                            anchors.left:           parent.left
                            anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
                            anchors.right:          parent.right
                            anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
                            anchors.verticalCenter: parent.verticalCenter
                            spacing:                ScreenTools.defaultFontPixelWidth

                            QGCLabel {
                                Layout.fillWidth:   true
                                text:               modelData
                                color:              _fault ? qgcPal.text : Qt.alpha(qgcPal.text, 0.7)
                            }

                            QGCLabel {
                                text:   control._sensorStatus[index]
                                color:  _fault ? qgcPal.colorRed : Qt.alpha(qgcPal.text, 0.5)
                            }

                            QGCColoredImage {
                                Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 0.9
                                Layout.preferredWidth:  Layout.preferredHeight
                                source:                 "/InstrumentValueIcons/cheveron-right.svg"
                                color:                  Qt.alpha(qgcPal.text, 0.4)
                                sourceSize.height:      height
                                fillMode:               Image.PreserveAspectFit
                                mipmap:                 true
                            }
                        }

                        QGCMouseArea {
                            id:             sensorMouseArea
                            anchors.fill:   parent
                            hoverEnabled:   !ScreenTools.isMobile
                            onClicked:      mainLayout.openVehicleSetup()
                        }
                    }
                }

                OverlayDisclosureRow {
                    objectName: "sensorDisclosure"
                    visible:    statusSummary.normalCount > 0
                    text:       mainLayout._showAllSensors ? qsTr("Show Less")
                                                           : qsTr("Show %n More", "", statusSummary.normalCount)
                    onClicked:  mainLayout._showAllSensors = !mainLayout._showAllSensors
                }
            }

            SettingsGroupLayout {
                popoverStyle:       true
                heading:            qsTr("Overall Status")
                visible:            _healthAndArmingChecksSupported && _activeVehicle.healthAndArmingCheckReport.problemsForCurrentMode.count > 0

                Repeater {
                    model:      _activeVehicle ? _activeVehicle.healthAndArmingCheckReport.problemsForCurrentMode : null
                    delegate:   listdelegate
                }
            }

            SettingsGroupLayout {
                popoverStyle:       true
                cardStyle:          true
                insetContent:       false
                showDividers:       false
                Layout.fillWidth:   true

                OverlayMenuItem {
                    objectName:         "vehicleSetupItem"
                    Layout.fillWidth:   true
                    disclosure:         true
                    text:               qsTr("Vehicle Setup")
                    onClicked:          mainLayout.openVehicleSetup()
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

