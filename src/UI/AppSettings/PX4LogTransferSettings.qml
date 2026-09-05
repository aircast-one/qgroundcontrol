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
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.ScreenTools

SettingsPage {
    property bool _showMavlinkLog:              QGroundControl.corePlugin.options.showPX4LogTransferOptions
    property var  _activeVehicle:               QGroundControl.multiVehicleManager.activeVehicle
    property bool _isPX4:                       _activeVehicle ? _activeVehicle.px4Firmware : true
    property Fact _disableDataPersistenceFact:  QGroundControl.settingsManager.appSettings.disableAllPersistence
    property bool _disableDataPersistence:      _disableDataPersistenceFact ? _disableDataPersistenceFact.rawValue : false
    property var  _mavlinkLogManager:           _activeVehicle ? _activeVehicle.mavlinkLogManager : null
    property int  _selectedCount:               0
    property bool _uploadedSelected:            false

    readonly property real _fieldWidth: ScreenTools.defaultFontPixelWidth * 28

    function _logFiles() {
        return Array.from({ length: _mavlinkLogManager.logFiles.count }, (_, i) => _mavlinkLogManager.logFiles.get(i))
    }

    function _selectAll(selected) {
        _logFiles().forEach(f => f.selected = selected)
    }

    function _showEmptyEmailDialog() {
        mainWindow.showMessageDialog(qsTr("MAVLink Logging"), qsTr("Please enter an email address before uploading MAVLink log files."))
    }

    Connections {
        target: _mavlinkLogManager
        onSelectedCountChanged: {
            const selected = _logFiles().filter(f => f.selected)
            _selectedCount = selected.length
            _uploadedSelected = selected.some(f => f.uploaded)
        }
    }

    QGCLabel {
        text:       qsTr("Connect a PX4 vehicle to manage logs")
        color:      QGroundControl.globalPalette.colorGrey
        visible:    _showMavlinkLog && !_mavlinkLogManager
    }

    Loader {
        id:                 logLoader
        Layout.fillWidth:   true
        active:             _showMavlinkLog && _isPX4 && _mavlinkLogManager

        sourceComponent: ColumnLayout {
            spacing: ScreenTools.defaultFontPixelHeight

            function saveItems() {
                _mavlinkLogManager.videoURL = videoUrlField.text
                _mavlinkLogManager.feedback = feedbackTextArea.text
                _mavlinkLogManager.emailAddress = emailField.text
                _mavlinkLogManager.description = descField.text
                _mavlinkLogManager.uploadURL = urlField.text
                if (autoUploadCheck.checked && _mavlinkLogManager.emailAddress === "") {
                    autoUploadCheck.checked = false
                } else {
                    _mavlinkLogManager.enableAutoUpload = autoUploadCheck.checked
                }
            }

            SettingsGroupLayout {
                Layout.fillWidth:   true
                heading:            qsTr("MAVLink 2.0 Logging")
                description:        qsTr("PX4 Pro only")

                RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth

                    QGCLabel {
                        Layout.fillWidth:   true
                        text:               qsTr("Logging")
                    }

                    QGCButton {
                        text:       qsTr("Start")
                        enabled:    !_mavlinkLogManager.logRunning && _mavlinkLogManager.canStartLog && !_disableDataPersistence
                        onClicked:  _mavlinkLogManager.startLogging()
                    }

                    QGCButton {
                        text:       qsTr("Stop")
                        enabled:    _mavlinkLogManager.logRunning && !_disableDataPersistence
                        onClicked:  _mavlinkLogManager.stopLogging()
                    }
                }

                QGCCheckBoxSlider {
                    Layout.fillWidth:   true
                    text:               qsTr("Start logging automatically")
                    checked:            _mavlinkLogManager.enableAutoStart
                    enabled:            !_disableDataPersistence
                    onClicked:          _mavlinkLogManager.enableAutoStart = checked
                }
            }

            SettingsGroupLayout {
                Layout.fillWidth:   true
                heading:            qsTr("Log Upload")

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Email Address") }
                    QGCTextField {
                        id:                     emailField
                        Layout.preferredWidth:  _fieldWidth
                        text:                   _mavlinkLogManager.emailAddress
                        enabled:                !_disableDataPersistence
                        inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhEmailCharactersOnly
                        onEditingFinished:      saveItems()
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Default Description") }
                    QGCTextField {
                        id:                     descField
                        Layout.preferredWidth:  _fieldWidth
                        text:                   _mavlinkLogManager.description
                        enabled:                !_disableDataPersistence
                        onEditingFinished:      saveItems()
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Upload URL") }
                    QGCTextField {
                        id:                     urlField
                        Layout.preferredWidth:  _fieldWidth
                        text:                   _mavlinkLogManager.uploadURL
                        enabled:                !_disableDataPersistence
                        inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhUrlCharactersOnly
                        onEditingFinished:      saveItems()
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Video URL") }
                    QGCTextField {
                        id:                     videoUrlField
                        Layout.preferredWidth:  _fieldWidth
                        text:                   _mavlinkLogManager.videoURL
                        enabled:                !_disableDataPersistence
                        inputMethodHints:       Qt.ImhNoAutoUppercase | Qt.ImhUrlCharactersOnly
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Wind Speed") }
                    QGCComboBox {
                        id:         windCombo
                        enabled:    !_disableDataPersistence
                        textRole:   "text"
                        model: ListModel {
                            id: windItems
                            ListElement { text: qsTr("Not Set"); value: -1 }
                            ListElement { text: qsTr("Calm");    value: 0 }
                            ListElement { text: qsTr("Breeze");  value: 5 }
                            ListElement { text: qsTr("Gale");    value: 8 }
                            ListElement { text: qsTr("Storm");   value: 10 }
                        }
                        onActivated: (index) => {
                            saveItems()
                            _mavlinkLogManager.windSpeed = windItems.get(index).value
                        }
                        Component.onCompleted: currentIndex = Math.max(0, _valueIndex(windItems, _mavlinkLogManager.windSpeed))
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; text: qsTr("Flight Rating") }
                    QGCComboBox {
                        id:         ratingCombo
                        enabled:    !_disableDataPersistence
                        textRole:   "text"
                        model: ListModel {
                            id: ratingItems
                            ListElement { text: qsTr("Not Set");                            value: "notset" }
                            ListElement { text: qsTr("Crashed (Pilot Error)");              value: "crash_pilot" }
                            ListElement { text: qsTr("Crashed (Software or Hardware Issue)"); value: "crash_sw_hw" }
                            ListElement { text: qsTr("Unsatisfactory");                     value: "unsatisfactory" }
                            ListElement { text: qsTr("Good");                               value: "good" }
                            ListElement { text: qsTr("Great");                              value: "great" }
                        }
                        onActivated: (index) => {
                            saveItems()
                            _mavlinkLogManager.rating = ratingItems.get(index).value
                        }
                        Component.onCompleted: currentIndex = Math.max(0, _valueIndex(ratingItems, _mavlinkLogManager.rating))
                    }
                }

                RowLayout {
                    Layout.fillWidth:   true
                    QGCLabel { Layout.fillWidth: true; Layout.alignment: Qt.AlignTop; text: qsTr("Additional Feedback") }
                    TextArea {
                        id:                     feedbackTextArea
                        Layout.preferredWidth:  _fieldWidth
                        Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 4
                        font.pointSize:         ScreenTools.defaultFontPointSize
                        text:                   _mavlinkLogManager.feedback
                        enabled:                !_disableDataPersistence
                        color:                  QGroundControl.globalPalette.textFieldText
                        background:             Rectangle { color: QGroundControl.globalPalette.textField; radius: ScreenTools.buttonBorderRadius }
                    }
                }

                QGCCheckBoxSlider {
                    Layout.fillWidth:   true
                    text:               qsTr("Make logs public")
                    checked:            _mavlinkLogManager.publicLog
                    enabled:            !_disableDataPersistence
                    onClicked:          _mavlinkLogManager.publicLog = checked
                }

                QGCCheckBoxSlider {
                    id:                 autoUploadCheck
                    Layout.fillWidth:   true
                    text:               qsTr("Upload logs automatically")
                    checked:            _mavlinkLogManager.enableAutoUpload
                    enabled:            !_disableDataPersistence
                    onClicked: {
                        saveItems()
                        if (checked && _mavlinkLogManager.emailAddress === "") {
                            _showEmptyEmailDialog()
                        }
                    }
                }

                QGCCheckBoxSlider {
                    Layout.fillWidth:   true
                    text:               qsTr("Delete logs after upload")
                    checked:            _mavlinkLogManager.deleteAfterUpload
                    enabled:            autoUploadCheck.checked && !_disableDataPersistence
                    onClicked:          _mavlinkLogManager.deleteAfterUpload = checked
                }
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Saved Log Files")
        visible:            _showMavlinkLog && _mavlinkLogManager

        Repeater {
            model: _mavlinkLogManager ? _mavlinkLogManager.logFiles : []

            RowLayout {
                Layout.fillWidth:   true
                spacing:            ScreenTools.defaultFontPixelWidth * 2

                QGCCheckBox {
                    checked:    object.selected
                    enabled:    !object.writing && !object.uploading
                    onClicked:  object.selected = checked
                }

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               object.name
                    color:              object.writing ? QGroundControl.globalPalette.warningText : QGroundControl.globalPalette.text
                }

                QGCLabel {
                    text:       Number(object.size).toLocaleString(Qt.locale(), 'f', 0)
                    color:      QGroundControl.globalPalette.colorGrey
                    visible:    !object.uploading && !object.uploaded
                }

                QGCLabel {
                    text:       qsTr("Uploaded")
                    color:      QGroundControl.globalPalette.colorGrey
                    visible:    object.uploaded
                }

                ProgressBar {
                    Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 20
                    visible:                object.uploading && !object.uploaded
                    from:                   0
                    to:                     100
                    value:                  object.progress * 100.0
                }
            }
        }

        QGCLabel {
            Layout.preferredHeight: ScreenTools.settingsRowHeight
            verticalAlignment:      Text.AlignVCenter
            text:                   qsTr("No log files")
            color:                  QGroundControl.globalPalette.colorGrey
            visible:                _mavlinkLogManager.logFiles.count === 0
        }

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth

            property bool _idle: _mavlinkLogManager && !_mavlinkLogManager.uploading && !_mavlinkLogManager.logRunning

            Item { Layout.fillWidth: true }

            QGCButton {
                text:       qsTr("Select All")
                enabled:    parent._idle
                onClicked:  _selectAll(true)
            }

            QGCButton {
                text:       qsTr("Select None")
                enabled:    parent._idle
                onClicked:  _selectAll(false)
            }

            QGCButton {
                text:       qsTr("Delete…")
                enabled:    _selectedCount > 0 && parent._idle
                onClicked:  mainWindow.showMessageDialog(qsTr("Delete Selected Log Files"), qsTr("Delete the selected log files?"), Dialog.Ok | Dialog.Cancel, function() { _mavlinkLogManager.deleteLog() })
            }

            QGCButton {
                text:       qsTr("Upload…")
                enabled:    _selectedCount > 0 && parent._idle && !_uploadedSelected
                visible:    !_mavlinkLogManager || !_mavlinkLogManager.uploading
                onClicked: {
                    logLoader.item.saveItems()
                    if (_mavlinkLogManager.emailAddress === "") {
                        _showEmptyEmailDialog()
                    } else {
                        mainWindow.showMessageDialog(qsTr("Upload Selected Log Files"), qsTr("Upload the selected log files?"), Dialog.Ok | Dialog.Cancel, function() { _mavlinkLogManager.uploadLog() })
                    }
                }
            }

            QGCButton {
                text:       qsTr("Cancel Upload…")
                visible:    _mavlinkLogManager && _mavlinkLogManager.uploading
                onClicked:  mainWindow.showMessageDialog(qsTr("Cancel Upload"), qsTr("Cancel the upload in progress?"), Dialog.Ok | Dialog.Cancel, function() { _mavlinkLogManager.cancelUpload() })
            }
        }
    }

    function _valueIndex(listModel, value) {
        return Array.from({ length: listModel.count }, (_, i) => listModel.get(i).value).indexOf(value)
    }
}
