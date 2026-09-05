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
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager
import QGroundControl.Palette
import QGroundControl.Controllers

SettingsPage {
    property var    _settingsManager:                       QGroundControl.settingsManager
    property var    _flyViewSettings:                       _settingsManager.flyViewSettings
    property var    _mavlinkActionsSettings:                _settingsManager.mavlinkActionsSettings
    property Fact   _virtualJoystick:                       _settingsManager.appSettings.virtualJoystick
    property Fact   _virtualJoystickAutoCenterThrottle:     _settingsManager.appSettings.virtualJoystickAutoCenterThrottle
    property Fact   _virtualJoystickLeftHandedMode:         _settingsManager.appSettings.virtualJoystickLeftHandedMode
    property Fact   _enableMultiVehiclePanel:               _settingsManager.appSettings.enableMultiVehiclePanel
    property Fact   _showAdditionalIndicatorsCompass:       _flyViewSettings.showAdditionalIndicatorsCompass
    property Fact   _lockNoseUpCompass:                     _flyViewSettings.lockNoseUpCompass
    property Fact   _guidedMinimumAltitude:                 _flyViewSettings.guidedMinimumAltitude
    property Fact   _guidedMaximumAltitude:                 _flyViewSettings.guidedMaximumAltitude
    property Fact   _maxGoToLocationDistance:               _flyViewSettings.maxGoToLocationDistance
    property Fact   _forwardFlightGoToLocationLoiterRad:    _flyViewSettings.forwardFlightGoToLocationLoiterRad
    property Fact   _goToLocationRequiresConfirmInGuided:   _flyViewSettings.goToLocationRequiresConfirmInGuided
    property var    _viewer3DSettings:                      _settingsManager.viewer3DSettings
    property Fact   _viewer3DEnabled:                       _viewer3DSettings.enabled
    property Fact   _viewer3DOsmFilePath:                   _viewer3DSettings.osmFilePath
    property Fact   _viewer3DBuildingLevelHeight:           _viewer3DSettings.buildingLevelHeight
    property Fact   _viewer3DAltitudeBias:                  _viewer3DSettings.altitudeBias

    QGCFileDialogController { id: fileController }

    function mavlinkActionList() {
        var fileModel = fileController.getFiles(_settingsManager.appSettings.mavlinkActionsSavePath, "*.json")
        fileModel.unshift(qsTr("<None>"))
        return fileModel
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true

        FactCheckBoxSlider {
            id:                 useCheckList
            Layout.fillWidth:   true
            text:               qsTr("Use preflight checklist")
            fact:               _useChecklist
            visible:            _useChecklist.visible && QGroundControl.corePlugin.options.preFlightChecklistUrl.toString().length
            property Fact _useChecklist:      _settingsManager.appSettings.useChecklist
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Enforce preflight checklist")
            fact:               _enforceChecklist
            enabled:            _settingsManager.appSettings.useChecklist.value
            visible:            useCheckList.visible && _enforceChecklist.visible
            property Fact _enforceChecklist: _settingsManager.appSettings.enforceChecklist
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show multi-vehicle panel")
            fact:               _enableMultiVehiclePanel
            visible:            _enableMultiVehiclePanel.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Keep map centered on vehicle")
            fact:               _keepMapCenteredOnVehicle
            visible:            _keepMapCenteredOnVehicle.visible
            property Fact _keepMapCenteredOnVehicle: _flyViewSettings.keepMapCenteredOnVehicle
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show telemetry log replay status bar")
            fact:               _showLogReplayStatusBar
            visible:            _showLogReplayStatusBar.visible
            property Fact _showLogReplayStatusBar: _flyViewSettings.showLogReplayStatusBar
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show simple camera controls (DIGICAM_CONTROL)")
            visible:            _showDumbCameraControl.visible
            fact:               _showDumbCameraControl

            property Fact _showDumbCameraControl: _flyViewSettings.showSimpleCameraControl
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show photo/video recording control")
            visible:            _showPhotoVideoControl.visible
            fact:               _showPhotoVideoControl

            property Fact _showPhotoVideoControl: _flyViewSettings.showPhotoVideoControl
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Update return to home position based on device location")
            fact:               _updateHomePosition
            visible:            _updateHomePosition.visible
            property Fact _updateHomePosition: _flyViewSettings.updateHomePosition
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Guided Commands")
        visible:            _guidedMinimumAltitude.visible || _guidedMaximumAltitude.visible ||
                            _maxGoToLocationDistance.visible || _forwardFlightGoToLocationLoiterRad.visible ||
                            _goToLocationRequiresConfirmInGuided.visible

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Minimum Altitude")
            fact:               _guidedMinimumAltitude
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Maximum Altitude")
            fact:               _guidedMaximumAltitude
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Go To Location Max Distance")
            fact:               _maxGoToLocationDistance
            visible:            fact.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Loiter Radius in Forward Flight Guided Mode")
            fact:               _forwardFlightGoToLocationLoiterRad
            visible:            fact.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Confirm before Go To Location in guided mode")
            fact:               _goToLocationRequiresConfirmInGuided
            visible:            fact.visible
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:       true
        Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 35
        heading:                qsTr("MAVLink Actions")
        description:            qsTr("Action JSON files should be created in the '%1' folder.").arg(QGroundControl.settingsManager.appSettings.mavlinkActionsSavePath)

        LabelledComboBox {
            Layout.fillWidth:   true
            label:              qsTr("Fly View Actions")
            model:              mavlinkActionList()
            onActivated:        (index) => index == 0 ? _mavlinkActionsSettings.flyViewActionsFile.rawValue = "" : _mavlinkActionsSettings.flyViewActionsFile.rawValue = comboBox.currentText
            enabled:            model.length > 1

            Component.onCompleted: {
                var index = comboBox.find(_mavlinkActionsSettings.flyViewActionsFile.valueString)
                comboBox.currentIndex = index == -1 ? 0 : index
            }
        }

        LabelledComboBox {
            Layout.fillWidth:   true
            label:              qsTr("Joystick Actions")
            model:              mavlinkActionList()
            onActivated:        (index) => index == 0 ? _mavlinkActionsSettings.joystickActionsFile.rawValue = "" : _mavlinkActionsSettings.joystickActionsFile.rawValue = comboBox.currentText
            enabled:            model.length > 1

            Component.onCompleted: {
                var index = comboBox.find(_mavlinkActionsSettings.joystickActionsFile.valueString)
                comboBox.currentIndex = index == -1 ? 0 : index
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Virtual Joystick")
        visible:            _virtualJoystick.visible || _virtualJoystickAutoCenterThrottle.visible || _virtualJoystickLeftHandedMode.visible

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Enabled")
            visible:            _virtualJoystick.visible
            fact:               _virtualJoystick
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Auto-center throttle")
            visible:            _virtualJoystickAutoCenterThrottle.visible
            enabled:            _virtualJoystick.rawValue
            fact:               _virtualJoystickAutoCenterThrottle
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Left-handed mode (swap sticks)")
            visible:            _virtualJoystickLeftHandedMode.visible
            enabled:            _virtualJoystick.rawValue
            fact:               _virtualJoystickLeftHandedMode
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Instrument Panel")
        visible:            _showAdditionalIndicatorsCompass.visible || _lockNoseUpCompass.visible

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Show additional heading indicators on Compass")
            visible:            _showAdditionalIndicatorsCompass.visible
            fact:               _showAdditionalIndicatorsCompass
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Lock compass nose-up")
            visible:            _lockNoseUpCompass.visible
            fact:               _lockNoseUpCompass
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:       true
        Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 40
        heading:                qsTr("Camera & Gimbal Control")
        description:            qsTr("Map the vehicle's RC channels once here and the fly view gains camera controls: drag the video to aim, tilt and zoom sliders on the edges. Leave a channel at 0 to hide its control. Channel numbers come from the vehicle's RCn_OPTION setup.")

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Gimbal tilt channel")
            fact:               _flyViewSettings.gimbalTiltChannel
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Gimbal pan channel")
            fact:               _flyViewSettings.gimbalPanChannel
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Camera zoom channel")
            fact:               _flyViewSettings.cameraZoomChannel
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Camera light channel")
            fact:               _flyViewSettings.cameraLightChannel
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Camera record channel")
            fact:               _flyViewSettings.cameraRecordChannel
        }
    }

    SettingsGroupLayout {
        id:                     rcControlsGroup
        objectName:             "rcControlsGroup"
        Layout.fillWidth:       true
        Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 65
        heading:                qsTr("Custom RC Controls")
        description:            qsTr("Add an on-screen control for any RC channel: a slider, a latching toggle switch, a 3-position switch, or a spring-loaded momentary switch. Sliders sweep 1000-2000 µs; switches move between 1000/1500/2000 µs. Controls are greyed out until a vehicle connects. Long-press any control in the fly view to arrange them.")

        readonly property int _channelCount: 18

        readonly property var _typeOptions: [
            { value: "slider",    label: qsTr("Slider") },
            { value: "button",    label: qsTr("Toggle Switch") },
            { value: "switch3",   label: qsTr("3-Position Switch") },
            { value: "momentary", label: qsTr("Momentary Switch") }
        ]

        function _typeIndex(type) {
            const i = _typeOptions.findIndex((option) => option.value === type)
            return i < 0 ? 0 : i
        }

        property var _controls: _parse(_flyViewSettings.rcControls.rawValue)

        readonly property var _cameraChannels: [
            { channel: _flyViewSettings.gimbalTiltChannel.rawValue,  owner: qsTr("Gimbal tilt") },
            { channel: _flyViewSettings.gimbalPanChannel.rawValue,   owner: qsTr("Gimbal pan") },
            { channel: _flyViewSettings.cameraZoomChannel.rawValue,  owner: qsTr("Camera zoom") },
            { channel: _flyViewSettings.cameraLightChannel.rawValue, owner: qsTr("Camera light") },
            { channel: _flyViewSettings.cameraRecordChannel.rawValue, owner: qsTr("Camera record") }
        ]

        function _channelNamesFor(rowIndex) {
            return Array.from({ length: _channelCount }, (_, i) => {
                const owner = _usedBy(i + 1, rowIndex)
                return owner === "" ? String(i + 1) : qsTr("%1 · %2").arg(i + 1).arg(owner)
            })
        }

        function _parse(json) {
            try { return JSON.parse(json) } catch (e) { return [] }
        }

        function _usedBy(channel, beforeIndex) {
            const camera = _cameraChannels.find((mapping) => mapping.channel === channel)
            if (camera) {
                return camera.owner
            }
            const rowIndex = _controls.findIndex((control, i) => i < beforeIndex && control.channel === channel)
            return rowIndex < 0 ? "" : (_controls[rowIndex].label || qsTr("control %1").arg(rowIndex + 1))
        }

        function _firstFreeChannel() {
            const free = Array.from({ length: _channelCount }, (_, i) => i + 1)
                              .find((channel) => _usedBy(channel, _controls.length) === "")
            return free === undefined ? 1 : free
        }

        function _save(controls) {
            _flyViewSettings.rcControls.rawValue = JSON.stringify(controls)
        }

        function _update(index, patch) {
            _save(_controls.map((control, i) => i === index ? Object.assign({}, control, patch) : control))
        }

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth
            visible:            rcControlsGroup._controls.length > 0

            QGCLabel { Layout.fillWidth: true;                                       text: qsTr("Name");        color: QGroundControl.globalPalette.colorGrey; font.pointSize: ScreenTools.smallFontPointSize }
            QGCLabel { Layout.preferredWidth: ScreenTools.defaultFontPixelWidth * 16; text: qsTr("Channel");     color: QGroundControl.globalPalette.colorGrey; font.pointSize: ScreenTools.smallFontPointSize }
            QGCLabel { Layout.preferredWidth: ScreenTools.defaultFontPixelWidth * 16; text: qsTr("Type");        color: QGroundControl.globalPalette.colorGrey; font.pointSize: ScreenTools.smallFontPointSize }
            QGCLabel { Layout.preferredWidth: ScreenTools.defaultFontPixelWidth * 12; text: qsTr("Orientation"); color: QGroundControl.globalPalette.colorGrey; font.pointSize: ScreenTools.smallFontPointSize }
            Item     { Layout.preferredWidth: removeSizer.width }

            QGCButton { id: removeSizer; text: qsTr("Remove"); visible: false }
        }

        Repeater {
            model: rcControlsGroup._controls

            delegate: ColumnLayout {
                id: rcControlRow

                required property int index
                required property var modelData

                readonly property string conflict: rcControlsGroup._usedBy(modelData.channel, index)

                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelHeight / 4

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth

                    QGCTextField {
                        objectName:         "rcControlLabel" + rcControlRow.index
                        Layout.fillWidth:   true
                        text:               rcControlRow.modelData.label
                        placeholderText:    qsTr("CH%1").arg(rcControlRow.modelData.channel)
                        onEditingFinished:  rcControlsGroup._update(rcControlRow.index, { label: text })
                    }

                    QGCComboBox {
                        objectName:             "rcControlChannel" + rcControlRow.index
                        Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 16
                        model:                  rcControlsGroup._channelNamesFor(rcControlRow.index)
                        currentIndex:           Math.max(0, rcControlRow.modelData.channel - 1)
                        onActivated:            (i) => rcControlsGroup._update(rcControlRow.index, { channel: i + 1 })
                    }

                    QGCComboBox {
                        objectName:             "rcControlType" + rcControlRow.index
                        Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 16
                        model:                  rcControlsGroup._typeOptions.map((option) => option.label)
                        currentIndex:           rcControlsGroup._typeIndex(rcControlRow.modelData.type)
                        onActivated:            (i) => rcControlsGroup._update(rcControlRow.index, { type: rcControlsGroup._typeOptions[i].value })
                    }

                    QGCComboBox {
                        objectName:             "rcControlOrientation" + rcControlRow.index
                        Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 12
                        visible:                (rcControlRow.modelData.type || "slider") === "slider"
                        model:                  [ qsTr("Vertical"), qsTr("Horizontal") ]
                        currentIndex:           rcControlRow.modelData.orientation === "horizontal" ? 1 : 0
                        onActivated:            (i) => rcControlsGroup._update(rcControlRow.index, { orientation: i === 1 ? "horizontal" : "vertical" })
                    }

                    QGCButton {
                        objectName: "rcControlRemove" + rcControlRow.index
                        text:       qsTr("Remove")
                        onClicked:  rcControlsGroup._save(rcControlsGroup._controls.filter((_, i) => i !== rcControlRow.index))
                    }
                }

                QGCLabel {
                    objectName:         "rcControlConflict" + rcControlRow.index
                    Layout.fillWidth:   true
                    visible:            rcControlRow.conflict !== ""
                    color:              QGroundControl.globalPalette.colorOrange
                    font.pointSize:     ScreenTools.smallFontPointSize
                    wrapMode:           Text.WordWrap
                    text:               qsTr("Channel %1 is already driven by %2. Only the first control on a channel is shown in the fly view.")
                                            .arg(rcControlRow.modelData.channel).arg(rcControlRow.conflict)
                }
            }
        }

        QGCButton {
            objectName: "rcControlsAddButton"
            text:       qsTr("Add Control")
            onClicked:  rcControlsGroup._save([...rcControlsGroup._controls,
                                               { label: "", channel: rcControlsGroup._firstFreeChannel(), type: "slider" }])
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("3D View")
        visible:            _viewer3DSettings.visible

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Enabled")
            fact:               _viewer3DEnabled
            visible:            _viewer3DEnabled.visible
        }

        ColumnLayout{
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth
            enabled:            _viewer3DEnabled.rawValue
            visible:            _viewer3DOsmFilePath.rawValue

            RowLayout{
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelWidth

                QGCLabel {
                    wrapMode:   Text.WordWrap
                    visible:    true
                    text:       qsTr("3D Map File")
                }

                QGCTextField {
                    id:                 osmFileTextField
                    height:             ScreenTools.defaultFontPixelWidth * 4.5
                    unitsLabel:         ""
                    showUnits:          false
                    visible:            true
                    Layout.fillWidth:   true
                    readOnly:           true
                    text:               _viewer3DOsmFilePath.rawValue
                }
            }

            RowLayout{
                Layout.alignment:   Qt.AlignRight
                spacing:            ScreenTools.defaultFontPixelWidth

                QGCButton {
                    text: qsTr("Clear")

                    onClicked: {
                        osmFileTextField.text = "Please select an OSM file"
                        _viewer3DOsmFilePath.value = osmFileTextField.text
                    }
                }

                QGCButton {
                    text: qsTr("Choose…")

                    onClicked: {
                        var filename = _viewer3DOsmFilePath.rawValue;
                        const found = filename.match(/(.*)[\/\\]/);
                        if(found){
                            filename = found[1]||'';
                            fileDialog.folder = (filename[0] === "/")?(filename.slice(1)):(filename);
                        }
                        fileDialog.openForLoad()
                    }

                    QGCFileDialog {
                        id:             fileDialog
                        nameFilters:    [qsTr("OpenStreetMap files (*.osm)")]
                        title:          qsTr("Select map file")

                        onAcceptedForLoad: (file) => {
                                               osmFileTextField.text = file
                                               _viewer3DOsmFilePath.value = osmFileTextField.text
                        }
                    }
                }
            }
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Average Building Level Height")
            fact:               _viewer3DBuildingLevelHeight
            enabled:            _viewer3DEnabled.rawValue
            visible:            _viewer3DBuildingLevelHeight.visible
        }

        LabelledFactTextField {
            Layout.fillWidth:   true
            label:              qsTr("Vehicles Altitude Bias")
            fact:               _viewer3DAltitudeBias
            enabled:            _viewer3DEnabled.rawValue
            visible:            _viewer3DAltitudeBias.visible
        }
    }
}
