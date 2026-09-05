import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Vehicle
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.SettingsManager
import QGroundControl.Controllers

Item {
    id:                 valuesRect
    width:              availableWidth
    implicitHeight:     valuesColumn.implicitHeight
    height:             implicitHeight

    property var    _masterControler:               masterController
    property var    _missionController:             _masterControler.missionController
    property var    _controllerVehicle:             _masterControler.controllerVehicle
    property bool   _vehicleHasHomePosition:        _controllerVehicle.homePosition.isValid
    property bool   _showCruiseSpeed:               !_controllerVehicle.multiRotor
    property bool   _showHoverSpeed:                _controllerVehicle.multiRotor || _controllerVehicle.vtol
    property bool   _multipleFirmware:              !QGroundControl.singleFirmwareSupport
    property bool   _multipleVehicleTypes:          !QGroundControl.singleVehicleSupport
    property real   _fieldWidth:                    ScreenTools.defaultFontPixelWidth * 12
    property bool   _mobile:                        ScreenTools.isMobile
    property var    _savePath:                      QGroundControl.settingsManager.appSettings.missionSavePath
    property var    _fileExtension:                 QGroundControl.settingsManager.appSettings.missionFileExtension
    property var    _appSettings:                   QGroundControl.settingsManager.appSettings
    property bool   _waypointsOnlyMode:             QGroundControl.corePlugin.options.missionWaypointsOnly
    property bool   _showCameraSection:             (_waypointsOnlyMode || QGroundControl.corePlugin.showAdvancedUI) && !_controllerVehicle.apmFirmware
    property bool   _simpleMissionStart:            QGroundControl.corePlugin.options.showSimpleMissionStart
    property bool   _showFlightSpeed:               !_controllerVehicle.vtol && !_simpleMissionStart && !_controllerVehicle.apmFirmware
    property bool   _allowFWVehicleTypeSelection:   _noMissionItemsAdded && !globals.activeVehicle
    property bool   _showVehicleInfo:               !_waypointsOnlyMode
    property bool   _showLaunchPosition:            !_simpleMissionStart && !_vehicleHasHomePosition

    readonly property string _firmwareLabel:    qsTr("Firmware")
    readonly property string _vehicleLabel:     qsTr("Vehicle")
    readonly property real  _margin:            ScreenTools.defaultFontPixelWidth / 2

    QGCPalette { id: qgcPal }
    QGCFileDialogController { id: fileController }
    Component { id: altModeDialogComponent; AltModeDialog { } }

    Connections {
        target: _controllerVehicle
        function onSupportsTerrainFrameChanged() {
            if (!_controllerVehicle.supportsTerrainFrame && _missionController.globalAltitudeMode === QGroundControl.AltitudeModeTerrainFrame) {
                _missionController.globalAltitudeMode = QGroundControl.AltitudeModeCalcAboveTerrain
            }
        }
    }

    function _editAltitudeMode() {
        const removeModes = _controllerVehicle.supportsTerrainFrame ? [] : [QGroundControl.AltitudeModeTerrainFrame]
        const lockedModes = _noMissionItemsAdded
                                ? []
                                : [QGroundControl.AltitudeModeRelative,
                                   QGroundControl.AltitudeModeAbsolute,
                                   QGroundControl.AltitudeModeCalcAboveTerrain,
                                   QGroundControl.AltitudeModeTerrainFrame]
                                      .filter((mode) => mode !== _missionController.globalAltitudeMode)
        altModeDialogComponent.createObject(mainWindow, {
            rgRemoveModes:   [...removeModes, ...lockedModes],
            updateAltModeFn: (altMode) => { _missionController.globalAltitudeMode = altMode }
        }).open()
    }

    ColumnLayout {
        id:                 valuesColumn
        anchors.left:       parent.left
        anchors.right:      parent.right
        anchors.top:        parent.top
        spacing:            _margin

        SettingsGroupLayout {
            Layout.fillWidth:   true
            heading:            qsTr("Altitude")
            popoverStyle:       true
            cardStyle:          true

            RowLayout {
                Layout.fillWidth:       true
                Layout.preferredHeight: ScreenTools.settingsRowHeight
                spacing:                ScreenTools.defaultFontPixelWidth * 2

                QGCLabel {
                    Layout.fillWidth:   true
                    Layout.alignment:   Qt.AlignVCenter
                    text:               qsTr("Mode")
                }

                QGCLabel {
                    id:                 altModeLabel
                    Layout.alignment:   Qt.AlignVCenter
                    text:               QGroundControl.altitudeModeShortDescription(_missionController.globalAltitudeMode)
                    color:              qgcPal.colorGrey
                }

                QGCColoredImage {
                    Layout.alignment:   Qt.AlignVCenter
                    height:             ScreenTools.defaultFontPixelHeight / 2
                    width:              height
                    source:             "/res/DropArrow.svg"
                    color:              altModeLabel.color
                }

                TapHandler {
                    onTapped: valuesRect._editAltitudeMode()
                }
            }

            LabelledFactTextField {
                Layout.fillWidth:           true
                label:                      qsTr("Initial Waypoint")
                fact:                       QGroundControl.settingsManager.appSettings.defaultMissionItemAltitude
                textFieldPreferredWidth:    _fieldWidth
            }

            RowLayout {
                Layout.fillWidth:       true
                Layout.preferredHeight: ScreenTools.settingsRowHeight
                spacing:                ScreenTools.defaultFontPixelWidth * 2
                visible:                _showFlightSpeed

                QGCCheckBox {
                    id:                 flightSpeedCheckBox
                    Layout.fillWidth:   true
                    Layout.alignment:   Qt.AlignVCenter
                    text:               qsTr("Flight speed")
                    checked:            missionItem ? missionItem.speedSection.specifyFlightSpeed : false
                    onClicked:          if (missionItem) missionItem.speedSection.specifyFlightSpeed = checked
                }

                FactTextField {
                    Layout.preferredWidth:  _fieldWidth
                    Layout.alignment:       Qt.AlignVCenter
                    fact:                   missionItem ? missionItem.speedSection.flightSpeed : null
                    enabled:                flightSpeedCheckBox.checked
                    showFrame:              false
                    horizontalAlignment:    TextInput.AlignRight
                }
            }
        }

        Column {
            Layout.fillWidth:   true
            spacing:            _margin
            visible:            !_simpleMissionStart

            CameraSection {
                id:         cameraSection
                checked:    !_waypointsOnlyMode && missionItem !== null && missionItem.cameraSection.settingsSpecified
                visible:    _showCameraSection && missionItem !== null
            }

            QGCLabel {
                anchors.left:           parent.left
                anchors.right:          parent.right
                text:                   qsTr("Camera commands above take effect immediately at mission start.")
                wrapMode:               Text.WordWrap
                font.pointSize:         ScreenTools.smallFontPointSize
                color:                  qgcPal.colorGrey
                visible:                _showCameraSection && cameraSection.checked
            }
        }

        SettingsGroupLayout {
            Layout.fillWidth:   true
            heading:            qsTr("Vehicle")
            popoverStyle:       true
            cardStyle:          true
            visible:            _showVehicleInfo
            description:        (_showCruiseSpeed || _showHoverSpeed)
                                    ? qsTr("Speeds are used to estimate mission time only. They do not change the flight speed.")
                                    : ""

            LabelledFactComboBox {
                Layout.fillWidth:   true
                label:              _firmwareLabel
                fact:               QGroundControl.settingsManager.appSettings.offlineEditingFirmwareClass
                indexModel:         false
                visible:            _multipleFirmware && _allowFWVehicleTypeSelection
            }

            LabelledLabel {
                Layout.fillWidth:   true
                label:              _firmwareLabel
                labelText:          _controllerVehicle.firmwareTypeString
                visible:            _multipleFirmware && !_allowFWVehicleTypeSelection
            }

            LabelledFactComboBox {
                Layout.fillWidth:   true
                label:              _vehicleLabel
                fact:               QGroundControl.settingsManager.appSettings.offlineEditingVehicleClass
                indexModel:         false
                visible:            _multipleVehicleTypes && _allowFWVehicleTypeSelection
            }

            LabelledLabel {
                Layout.fillWidth:   true
                label:              _vehicleLabel
                labelText:          _controllerVehicle.vehicleTypeString
                visible:            _multipleVehicleTypes && !_allowFWVehicleTypeSelection
            }

            LabelledFactTextField {
                Layout.fillWidth:           true
                label:                      qsTr("Cruise speed")
                fact:                       QGroundControl.settingsManager.appSettings.offlineEditingCruiseSpeed
                textFieldPreferredWidth:    _fieldWidth
                visible:                    _showCruiseSpeed
            }

            LabelledFactTextField {
                Layout.fillWidth:           true
                label:                      qsTr("Hover speed")
                fact:                       QGroundControl.settingsManager.appSettings.offlineEditingHoverSpeed
                textFieldPreferredWidth:    _fieldWidth
                visible:                    _showHoverSpeed
            }
        }

        SettingsGroupLayout {
            Layout.fillWidth:   true
            heading:            qsTr("Launch Position")
            popoverStyle:       true
            cardStyle:          true
            visible:            _showLaunchPosition
            description:        qsTr("Actual position is set by the vehicle at flight time.")

            LabelledFactTextField {
                Layout.fillWidth:           true
                label:                      qsTr("Altitude")
                fact:                       missionItem ? missionItem.plannedHomePositionAltitude : null
                textFieldPreferredWidth:    _fieldWidth
            }

            LabelledButton {
                Layout.fillWidth:   true
                label:              qsTr("Position")
                buttonText:         qsTr("Set To Map Center")
                onClicked:          if (missionItem) missionItem.coordinate = map.center
            }
        }
    }
}
