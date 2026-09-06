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
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.Controllers

Item {
    id: _root

    property bool   showSensorCalibrationCompass:   true
    property bool   showSensorCalibrationGyro:      true
    property bool   showSensorCalibrationAccel:     true
    property bool   showSensorCalibrationLevel:     true
    property bool   showSensorCalibrationAirspeed:  true
    property bool   showSetOrientations:            true
    property bool   showNextButton:                 false

    signal nextButtonClicked

    readonly property string boardRotationText: qsTr("If the orientation is in the direction of flight, select ROTATION_NONE.")
    readonly property string compassRotationText: qsTr("If the orientation is in the direction of flight, select ROTATION_NONE.")

    readonly property string compassHelp:   qsTr("For Compass calibration you will need to rotate your vehicle through a number of positions.")
    readonly property string gyroHelp:      qsTr("For Gyroscope calibration you will need to place your vehicle on a surface and leave it still.")
    readonly property string accelHelp:     qsTr("For Accelerometer calibration you will need to place your vehicle on all six sides on a perfectly level surface and hold it still in each orientation for a few seconds.")
    readonly property string levelHelp:     qsTr("To level the horizon you need to place the vehicle in its level flight position and leave still.")
    readonly property string airspeedHelp:  qsTr("For Airspeed calibration you will need to keep your airspeed sensor out of any wind and then blow across the sensor. Do not touch the sensor or obstruct any holes during the calibration.")

    property string preCalibrationDialogType

    property string preCalibrationDialogHelp

    property Fact cal_mag0_id:      controller.getParameterFact(-1, "CAL_MAG0_ID")
    property Fact cal_mag1_id:      controller.getParameterFact(-1, "CAL_MAG1_ID")
    property Fact cal_mag2_id:      controller.getParameterFact(-1, "CAL_MAG2_ID")
    property Fact cal_mag0_rot:     controller.getParameterFact(-1, "CAL_MAG0_ROT")
    property Fact cal_mag1_rot:     controller.getParameterFact(-1, "CAL_MAG1_ROT")
    property Fact cal_mag2_rot:     controller.getParameterFact(-1, "CAL_MAG2_ROT")

    property Fact cal_gyro0_id:     controller.getParameterFact(-1, "CAL_GYRO0_ID")
    property Fact cal_acc0_id:      controller.getParameterFact(-1, "CAL_ACC0_ID")

    property Fact sens_board_rot:   controller.getParameterFact(-1, "SENS_BOARD_ROT")
    property Fact sens_dpres_off:   controller.getParameterFact(-1, "SENS_DPRES_OFF")

    property bool showCompass0Rot: cal_mag0_id.value > 0 && cal_mag0_rot.value >= 0
    property bool showCompass1Rot: cal_mag1_id.value > 0 && cal_mag1_rot.value >= 0
    property bool showCompass2Rot: cal_mag2_id.value > 0 && cal_mag2_rot.value >= 0

    property bool   _sensorsHaveFixedOrientation:       QGroundControl.corePlugin.options.sensorsHaveFixedOrientation
    property bool   _wifiReliableForCalibration:        QGroundControl.corePlugin.options.wifiReliableForCalibration
    property int    _buttonWidth:                       ScreenTools.defaultFontPixelWidth * 15
    readonly property real _calColumnWidthFraction:     0.6
    readonly property int  _calColumnButtonWidths:      3
    property string _calMagIdParamFormat:               "CAL_MAG#_ID"
    property string _calMagRotParamFormat:              "CAL_MAG#_ROT"
    property bool 	_allMagsDisabled:                   controller.parameterExists(-1, "SYS_HAS_MAG") ? controller.getParameterFact(-1, "SYS_HAS_MAG").value === 0 : false
    property bool   _boardOrientationChangeAllowed:     !_sensorsHaveFixedOrientation && setOrientationsDialogShowBoardOrientation
    property bool   _compassOrientationChangeAllowed:   !_sensorsHaveFixedOrientation
    property int    _arbitrarilyLargeMaxMagIndex:       50

    function currentMagParamCount() {
        if (_allMagsDisabled) {
            return 0
        } else {
            for (var index=0; index<_arbitrarilyLargeMaxMagIndex; index++) {
                var magIdParam = _calMagIdParamFormat.replace("#", index)
                if (!controller.parameterExists(-1, magIdParam)) {
                    return index
                }
            }
            console.warn("SensorSetup.qml:currentMagParamCount internal error")
            return -1
        }
    }

    function currentExternalMagCount() {
        if (_allMagsDisabled) {
            return 0
        } else {
            var externalMagCount = 0
            for (var index=0; index<_arbitrarilyLargeMaxMagIndex; index++) {
                var magIdParam = _calMagIdParamFormat.replace("#", index)
                if (controller.parameterExists(-1, magIdParam)) {
                    var calMagIdFact = controller.getParameterFact(-1, magIdParam)
                    var calMagRotFact = controller.getParameterFact(-1, _calMagRotParamFormat.replace("#", index))
                    if (calMagIdFact.value > 0 && calMagRotFact.value >= 0) {
                        externalMagCount++
                    }
                } else {
                    return externalMagCount
                }
            }
            console.warn("SensorSetup.qml:currentExternalMagCount internal error")
            return 0
        }
    }

    function orientationsButtonVisible() {
        if (_sensorsHaveFixedOrientation || !showSetOrientations) {
            return false
        } else if (_boardOrientationChangeAllowed) {
            return true
        } else if (_compassOrientationChangeAllowed && !_allMagsDisabled) {
            for (var index=0; index<_arbitrarilyLargeMaxMagIndex; index++) {
                var magIdParam = _calMagIdParamFormat.replace("#", index)
                if (controller.parameterExists(-1, magIdParam)) {
                    var calMagIdFact = controller.parameterExists(-1, magIdParam)
                    var calMagRotFact = controller.parameterExists(-1, _calMagRotParamFormat.replace("#", index))
                    if (calMagIdFact.value > 0 && calMagRotFact.value >= 0) {
                        return true
                    }
                }
            }
            return false
        } else {
            return false
        }
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    SensorsComponentController {
        id:                         controller
        statusLog:                  statusTextArea
        progressBar:                progressBar
        compassButton:              compassButton
        gyroButton:                 gyroButton
        accelButton:                accelButton
        airspeedButton:             airspeedButton
        levelButton:                levelButton
        cancelButton:               cancelButton
        setOrientationsButton:      setOrientationsButton
        orientationCalAreaHelpText: orientationCalAreaHelpText

        onResetStatusTextArea: statusLog.text = ""

        onMagCalComplete: {
            setOrientationsButton.visible               = orientationsButtonVisible()
            setOrientationsDialogShowBoardOrientation   = false
            setOrientationsDialogComponent.createObject(mainWindow, { title: qsTr("Compass Calibration Complete"), showRebootVehicleButton: true }).open()
        }

        onWaitingForCancelChanged: {
            if (controller.waitingForCancel) {
                waitForCancelDialogComponent.createObject(mainWindow).open()
            }
        }
    }

    Component.onCompleted: {
        var usingUDP = controller.usingUDPLink()
        if (usingUDP && !_wifiReliableForCalibration) {
            mainWindow.showMessageDialog(qsTr("Sensor Calibration"), qsTr("Performing sensor calibration over a WiFi connection is known to be unreliable. You should disconnect and perform calibration using a direct USB connection instead."))
        }
    }

    Component {
        id: waitForCancelDialogComponent

        QGCSimpleMessageDialog {
            title:      qsTr("Calibration Cancel")
            text:       qsTr("Waiting for Vehicle to response to Cancel. This may take a few seconds.")
            buttons:    0

            Connections {
                target: controller

                onWaitingForCancelChanged: {
                    if (!controller.waitingForCancel) {
                        close()
                    }
                }
            }
        }
    }

    Component {
        id: preCalibrationDialogComponent

        QGCPopupDialog {
            buttons: Dialog.Cancel | Dialog.Ok

            onAccepted: {
                if (preCalibrationDialogType == "gyro") {
                    controller.calibrateGyro()
                } else if (preCalibrationDialogType == "accel") {
                    controller.calibrateAccel()
                } else if (preCalibrationDialogType == "level") {
                    controller.calibrateLevel()
                } else if (preCalibrationDialogType == "compass") {
                    controller.calibrateCompass()
                } else if (preCalibrationDialogType == "airspeed") {
                    controller.calibrateAirspeed()
                }
            }

            ColumnLayout {
                spacing: ScreenTools.defaultFontPixelHeight

                QGCLabel {
                    Layout.minimumWidth:    ScreenTools.defaultFontPixelWidth * 50
                    Layout.preferredWidth:  innerColumn.width
                    wrapMode:               Text.WordWrap
                    text:                   preCalibrationDialogHelp
                }

                Column {
                    id:         innerColumn
                    spacing:    parent.spacing

                    QGCLabel {
                        id:         boardRotationHelp
                        wrapMode:   Text.WordWrap
                        visible:    !_sensorsHaveFixedOrientation && (preCalibrationDialogType == "accel" || preCalibrationDialogType == "compass")
                        text:       qsTr("Set autopilot orientation before calibrating.")
                    }

                    Column {
                        visible:    boardRotationHelp.visible
                        QGCLabel { text: qsTr("Autopilot Orientation") }

                        FactComboBox {
                            sizeToContents: true
                            fact:           sens_board_rot
                        }

                        QGCLabel {
                            wrapMode:   Text.WordWrap
                            text:       qsTr("ROTATION_NONE indicates component points in direction of flight.")
                        }
                    }

                    QGCLabel {
                        wrapMode:   Text.WordWrap
                        text:       qsTr("Click Ok to start calibration.")
                    }
                }
            }
        }
    }

    property bool setOrientationsDialogShowBoardOrientation:    true

    Component {
        id: setOrientationsDialogComponent

        QGCPopupDialog {
            buttons: Dialog.Ok

            property bool showRebootVehicleButton: true

            ColumnLayout {
                spacing: ScreenTools.defaultFontPixelHeight

                QGCLabel {
                    text:       qsTr("Reboot the vehicle prior to flight.")
                    visible:    showRebootVehicleButton
                }

                QGCButton {
                    text:       qsTr("Reboot Vehicle")
                    visible:    showRebootVehicleButton
                    onClicked: { controller.vehicle.rebootVehicle(); close() }
                }

                QGCLabel {
                    text:       qsTr("Adjust orientations as needed.\n\nROTATION_NONE indicates component points in direction of flight.")
                    visible:    _boardOrientationChangeAllowed || (_compassOrientationChangeAllowed && currentExternalMagCount() !== 0)
                }

                Column {
                    visible: _boardOrientationChangeAllowed

                    QGCLabel {
                        text: qsTr("Autopilot Orientation")
                    }

                    FactComboBox {
                        sizeToContents: true
                        fact:           sens_board_rot
                    }
                }

                Repeater {
                    model: _compassOrientationChangeAllowed ? currentMagParamCount() : 0

                    Column {
                        visible: calMagIdFact.value > 0 && calMagRotFact.value >= 0

                        property Fact calMagIdFact:     controller.getParameterFact(-1, _calMagIdParamFormat.replace("#", index))
                        property Fact calMagRotFact:    controller.getParameterFact(-1, _calMagRotParamFormat.replace("#", index))

                        QGCLabel {
                            text: qsTr("Mag %1 Orientation").arg(index)
                        }

                        FactComboBox {
                            sizeToContents: true
                            fact:           parent.calMagRotFact
                        }
                    }
                }
            }
        }
    }

    property string _calName

    component CalRow: PlanGroupRow {
        property var    needsCalibration: undefined
        property string hint:             ""
        property string blockedReason:    ""

        objectName:  "calRow" + text.replace(/\s/g, "")
        interactive: true
        enabled:     blockedReason === ""
        description: blockedReason === "" ? hint : blockedReason
        onClicked:   _calName = text

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            text:                   needsCalibration ? qsTr("Not calibrated") : qsTr("Calibrated")
            color:                  needsCalibration ? qgcPal.colorOrange : Qt.alpha(qgcPal.text, 0.5)
            visible:                needsCalibration !== undefined && blockedReason === ""
        }
    }

    Column {
        id:             calColumn
        anchors.left:   parent.left
        anchors.top:    parent.top
        width:          Math.min(parent.width * _calColumnWidthFraction, _buttonWidth * _calColumnButtonWidths)
        spacing:        ScreenTools.defaultFontPixelHeight * 0.6

        QGCLabel {
            text:               qsTr("CALIBRATION")
            font.pointSize:     ScreenTools.smallFontPointSize
            font.letterSpacing: 0.5
            color:              qgcPal.colorGrey
            leftPadding:        ScreenTools.defaultFontPixelWidth * 1.5
        }

        PlanGroupCard {
            width: parent.width

            CalRow {
                id:                 compassButton
                text:               qsTr("Compass")
                hint:               qsTr("Rotate the vehicle through several positions")
                needsCalibration:   cal_mag0_id.value === 0
                visible:            !_allMagsDisabled && QGroundControl.corePlugin.options.showSensorCalibrationCompass && showSensorCalibrationCompass
                onClicked: {
                    preCalibrationDialogType = "compass"
                    preCalibrationDialogHelp = compassHelp
                    preCalibrationDialogComponent.createObject(mainWindow, { title: qsTr("Calibrate Compass") }).open()
                }
            }

            CalRow {
                id:                 gyroButton
                text:               qsTr("Gyroscope")
                hint:               qsTr("Place the vehicle on a surface and leave it still")
                needsCalibration:   cal_gyro0_id.value === 0
                visible:            QGroundControl.corePlugin.options.showSensorCalibrationGyro && showSensorCalibrationGyro
                onClicked: {
                    preCalibrationDialogType = "gyro"
                    preCalibrationDialogHelp = gyroHelp
                    preCalibrationDialogComponent.createObject(mainWindow, { title: qsTr("Calibrate Gyro") }).open()
                }
            }

            CalRow {
                id:                 accelButton
                text:               qsTr("Accelerometer")
                hint:               qsTr("Hold the vehicle still on all six sides")
                needsCalibration:   cal_acc0_id.value === 0
                visible:            QGroundControl.corePlugin.options.showSensorCalibrationAccel && showSensorCalibrationAccel
                onClicked: {
                    preCalibrationDialogType = "accel"
                    preCalibrationDialogHelp = accelHelp
                    preCalibrationDialogComponent.createObject(mainWindow, { title: qsTr("Calibrate Accelerometer") }).open()
                }
            }

            CalRow {
                id:         levelButton
                text:       qsTr("Level Horizon")
                hint:       qsTr("Place the vehicle in its level flight position")
                blockedReason: cal_acc0_id.value !== 0 && cal_gyro0_id.value !== 0 ?
                                   "" : qsTr("Calibrate Accelerometer and Gyroscope first")
                visible:    QGroundControl.corePlugin.options.showSensorCalibrationLevel && showSensorCalibrationLevel
                onClicked: {
                    preCalibrationDialogType = "level"
                    preCalibrationDialogHelp = levelHelp
                    preCalibrationDialogComponent.createObject(mainWindow, { title: qsTr("Level Horizon") }).open()
                }
            }

            CalRow {
                id:                 airspeedButton
                text:               qsTr("Airspeed")
                hint:               qsTr("Shield the airspeed sensor from the wind")
                needsCalibration:   sens_dpres_off.value === 0
                visible:            vehicleComponent.airspeedCalSupported &&
                                    QGroundControl.corePlugin.options.showSensorCalibrationAirspeed &&
                                    showSensorCalibrationAirspeed
                onClicked: {
                    preCalibrationDialogType = "airspeed"
                    preCalibrationDialogHelp = airspeedHelp
                    preCalibrationDialogComponent.createObject(mainWindow, { title: qsTr("Calibrate Airspeed") }).open()
                }
            }

            CalRow {
                id:             setOrientationsButton
                text:           qsTr("Orientations")
                showChevron:    true
                visible:        orientationsButtonVisible()
                onClicked: {
                    setOrientationsDialogShowBoardOrientation = true
                    setOrientationsDialogComponent.createObject(mainWindow, { title: qsTr("Set Orientations"), showRebootVehicleButton: false }).open()
                }
            }
        }

        PlanGroupCard {
            width: parent.width

            PlanGroupRow {
                text:           qsTr("Factory reset")
                textColor:      qgcPal.colorRed
                interactive:    true
                onClicked:      controller.resetFactoryParameters()
            }
        }

        QGCButton {
            id:         nextButton
            text:       qsTr("Next")
            primary:    true
            visible:    showNextButton
            onClicked:  _root.nextButtonClicked()
        }
    }

    Rectangle {
        anchors.left:       calColumn.right
        anchors.leftMargin: ScreenTools.defaultFontPixelHeight
        anchors.right:      parent.right
        anchors.top:        parent.top
        visible:            statusTextArea.text !== ""
        height:             Math.min(parent.height, Math.max(ScreenTools.defaultFontPixelHeight * 5, statusTextArea.contentHeight + ScreenTools.defaultFontPixelHeight * 2))
        radius:             ScreenTools.defaultFontPixelHeight * 0.9
        color:              Qt.alpha(qgcPal.text, 0.055)

        TextArea {
            id:                 statusTextArea
            anchors.fill:       parent
            anchors.margins:    ScreenTools.defaultFontPixelHeight / 2
            readOnly:           true
            text:               ""
            color:              qgcPal.text
            background:         null
        }
    }

    SetupSheet {
        open:   cancelButton.enabled
        title:  _calName !== "" ? qsTr("Calibrating %1").arg(_calName) : qsTr("Calibrating")

        SetupProgressBar {
            id:                 progressBar
            Layout.fillWidth:   true
        }

        QGCLabel {
            id:                 orientationCalAreaHelpText
            Layout.fillWidth:   true
            wrapMode:           Text.WordWrap
            visible:            text !== ""
        }

        Flow {
            Layout.fillWidth:   true
            visible:            controller.showOrientationCalArea
            spacing:            ScreenTools.defaultFontPixelWidth

            property real indicatorWidth:   ScreenTools.defaultFontPixelWidth * 22
            property real indicatorHeight:  ScreenTools.defaultFontPixelHeight * 7

            VehicleRotationCal {
                width:              parent.indicatorWidth
                height:             parent.indicatorHeight
                visible:            controller.orientationCalDownSideVisible
                calValid:           controller.orientationCalDownSideDone
                calInProgress:      controller.orientationCalDownSideInProgress
                calInProgressText:  controller.orientationCalDownSideRotate ? qsTr("Rotate") : qsTr("Hold Still")
                imageSource:        controller.orientationCalDownSideRotate ? "qrc:///qmlimages/VehicleDownRotate.png" : "qrc:///qmlimages/VehicleDown.png"
            }
            VehicleRotationCal {
                width:              parent.indicatorWidth
                height:             parent.indicatorHeight
                visible:            controller.orientationCalUpsideDownSideVisible
                calValid:           controller.orientationCalUpsideDownSideDone
                calInProgress:      controller.orientationCalUpsideDownSideInProgress
                calInProgressText:  controller.orientationCalUpsideDownSideRotate ? qsTr("Rotate") : qsTr("Hold Still")
                imageSource:        controller.orientationCalUpsideDownSideRotate ? "qrc:///qmlimages/VehicleUpsideDownRotate.png" : "qrc:///qmlimages/VehicleUpsideDown.png"
            }
            VehicleRotationCal {
                width:              parent.indicatorWidth
                height:             parent.indicatorHeight
                visible:            controller.orientationCalNoseDownSideVisible
                calValid:           controller.orientationCalNoseDownSideDone
                calInProgress:      controller.orientationCalNoseDownSideInProgress
                calInProgressText:  controller.orientationCalNoseDownSideRotate ? qsTr("Rotate") : qsTr("Hold Still")
                imageSource:        controller.orientationCalNoseDownSideRotate ? "qrc:///qmlimages/VehicleNoseDownRotate.png" : "qrc:///qmlimages/VehicleNoseDown.png"
            }
            VehicleRotationCal {
                width:              parent.indicatorWidth
                height:             parent.indicatorHeight
                visible:            controller.orientationCalTailDownSideVisible
                calValid:           controller.orientationCalTailDownSideDone
                calInProgress:      controller.orientationCalTailDownSideInProgress
                calInProgressText:  controller.orientationCalTailDownSideRotate ? qsTr("Rotate") : qsTr("Hold Still")
                imageSource:        controller.orientationCalTailDownSideRotate ? "qrc:///qmlimages/VehicleTailDownRotate.png" : "qrc:///qmlimages/VehicleTailDown.png"
            }
            VehicleRotationCal {
                width:              parent.indicatorWidth
                height:             parent.indicatorHeight
                visible:            controller.orientationCalLeftSideVisible
                calValid:           controller.orientationCalLeftSideDone
                calInProgress:      controller.orientationCalLeftSideInProgress
                calInProgressText:  controller.orientationCalLeftSideRotate ? qsTr("Rotate") : qsTr("Hold Still")
                imageSource:        controller.orientationCalLeftSideRotate ? "qrc:///qmlimages/VehicleLeftRotate.png" : "qrc:///qmlimages/VehicleLeft.png"
            }
            VehicleRotationCal {
                width:              parent.indicatorWidth
                height:             parent.indicatorHeight
                visible:            controller.orientationCalRightSideVisible
                calValid:           controller.orientationCalRightSideDone
                calInProgress:      controller.orientationCalRightSideInProgress
                calInProgressText:  controller.orientationCalRightSideRotate ? qsTr("Rotate") : qsTr("Hold Still")
                imageSource:        controller.orientationCalRightSideRotate ? "qrc:///qmlimages/VehicleRightRotate.png" : "qrc:///qmlimages/VehicleRight.png"
            }
        }

        footer: [
            QGCButton {
                id:         cancelButton
                text:       qsTr("Cancel")
                enabled:    false
                onClicked:  controller.cancelCalibration()
            }
        ]
    }
}
