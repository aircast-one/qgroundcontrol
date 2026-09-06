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
import QGroundControl.SettingsManager
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager
import QGroundControl.Palette

SettingsPage {
    property var    _settingsManager:           QGroundControl.settingsManager
    property var    _appSettings:               _settingsManager.appSettings
    property var    _unitsSettings:             _settingsManager.unitsSettings
    property var    _brandImageSettings:        _settingsManager.brandImageSettings
    property Fact   _appFontPointSize:          _appSettings.appFontPointSize
    property Fact   _userBrandImageIndoor:      _brandImageSettings.userBrandImageIndoor
    property Fact   _userBrandImageOutdoor:     _brandImageSettings.userBrandImageOutdoor
    property Fact   _appSavePath:               _appSettings.savePath
    property Fact   _androidSaveToSDCard:       _appSettings.androidSaveToSDCard
    property bool   _resetPending:              false

    readonly property var _scalePercents: [ 80, 90, 100, 110, 125, 150, 175, 200 ]

    readonly property int _unitSystemIndex: _unitsSettings.unitSystem

    function _pointSizeForPercent(percent) {
        return Math.round(ScreenTools.platformFontPointSize * percent / 100)
    }

    function _nearestScaleIndex() {
        const current = _appFontPointSize.value / ScreenTools.platformFontPointSize * 100
        return _scalePercents
            .map((p, i) => ({ d: Math.abs(p - current), i }))
            .reduce((best, c) => c.d < best.d ? c : best).i
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true

        LabelledFactComboBox {
            label:      qsTr("Language")
            fact:       _appSettings.qLocaleLanguage
            indexModel: false
            visible:    _appSettings.qLocaleLanguage.visible
        }

        LabelledFactComboBox {
            label:          qsTr("Stream GCS Position")
            description:    qsTr("Sends this device's location to the vehicle so it can follow you")
            fact:           _appSettings.followTarget
            indexModel:     false
            visible:        _appSettings.followTarget.visible
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Mute all audio output")
            fact:               _audioMuted
            visible:            _audioMuted.visible
            property Fact _audioMuted: _appSettings.audioMuted
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Appearance")

        LabelledFactComboBox {
            label:      qsTr("Color Scheme")
            fact:       _appSettings.indoorPalette
            indexModel: false
            visible:    _appSettings.indoorPalette.visible
        }

        LabelledSlider {
            Layout.fillWidth:       true
            label:                  qsTr("Glass Frost")
            description:            qsTr("How much the map blurs behind panels and pills")
            from:                   0
            to:                     100
            stepSize:               5
            sliderPreferredWidth:   ScreenTools.defaultFontPixelWidth * 20
            value:                  _appSettings.overlayGlassFrost.rawValue
            visible:                _appSettings.overlayGlassFrost.visible
            onMoved:                (v) => _appSettings.overlayGlassFrost.rawValue = v
        }

        LabelledComboBox {
            label:          qsTr("UI Scaling")
            model:          _scalePercents.map(p => p + "%")
            currentIndex:   _nearestScaleIndex()
            visible:        _appFontPointSize.visible
            onActivated:    (index) => { _appFontPointSize.value = _pointSizeForPercent(_scalePercents[index]) }
        }

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth * 2
            visible:            _brandImageSettings.visible && !ScreenTools.isMobile && _userBrandImageIndoor.visible

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            0

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               qsTr("Indoor Brand Image")
                }
                QGCLabel {
                    Layout.fillWidth:   true
                    font.pointSize:     ScreenTools.smallFontPointSize
                    color:              QGroundControl.globalPalette.colorGrey
                    text:               _userBrandImageIndoor.valueString.replace("file:///", "")
                    elide:              Text.ElideMiddle
                    visible:            _userBrandImageIndoor.valueString.length > 0
                }
            }

            QGCButton {
                text:       qsTr("Choose…")
                onClicked:  userBrandImageIndoorBrowseDialog.openForLoad()

                QGCFileDialog {
                    id:                 userBrandImageIndoorBrowseDialog
                    title:              qsTr("Choose custom brand image file")
                    folder:             _userBrandImageIndoor.rawValue.replace("file:///", "")
                    selectFolder:       false
                    onAcceptedForLoad:  (file) => _userBrandImageIndoor.rawValue = "file:///" + file
                }
            }
        }

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth * 2
            visible:            _brandImageSettings.visible && !ScreenTools.isMobile && _userBrandImageOutdoor.visible

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            0

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               qsTr("Outdoor Brand Image")
                }
                QGCLabel {
                    Layout.fillWidth:   true
                    font.pointSize:     ScreenTools.smallFontPointSize
                    color:              QGroundControl.globalPalette.colorGrey
                    text:               _userBrandImageOutdoor.valueString.replace("file:///", "")
                    elide:              Text.ElideMiddle
                    visible:            _userBrandImageOutdoor.valueString.length > 0
                }
            }

            QGCButton {
                text:       qsTr("Choose…")
                onClicked:  userBrandImageOutdoorBrowseDialog.openForLoad()

                QGCFileDialog {
                    id:                 userBrandImageOutdoorBrowseDialog
                    title:              qsTr("Choose custom brand image file")
                    folder:             _userBrandImageOutdoor.rawValue.replace("file:///", "")
                    selectFolder:       false
                    onAcceptedForLoad:  (file) => _userBrandImageOutdoor.rawValue = "file:///" + file
                }
            }
        }

        LabelledButton {
            label:      ""
            buttonText: qsTr("Reset Images…")
            visible:    _brandImageSettings.visible && !ScreenTools.isMobile
            onClicked:  {
                _userBrandImageIndoor.rawValue = ""
                _userBrandImageOutdoor.rawValue = ""
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Files")
        description:        qsTr("Missions, telemetry logs and map tiles are written here. Removable storage keeps them off the device.")
        visible:            (_appSavePath.visible && !ScreenTools.isMobile) || _androidSaveToSDCard.visible

        RowLayout {
            Layout.fillWidth:   true
            spacing:            ScreenTools.defaultFontPixelWidth * 2
            visible:            _appSavePath.visible && !ScreenTools.isMobile

            ColumnLayout {
                Layout.fillWidth:   true
                spacing:            0

                QGCLabel { text: qsTr("Application Load/Save Path") }
                QGCLabel {
                    Layout.fillWidth:   true
                    font.pointSize:     ScreenTools.smallFontPointSize
                    color:              QGroundControl.globalPalette.colorGrey
                    text:               _appSavePath.rawValue === "" ? qsTr("<default location>") : _appSavePath.value
                    elide:              Text.ElideMiddle
                }
            }

            QGCButton {
                text:       qsTr("Choose…")
                onClicked:  savePathBrowseDialog.openForLoad()
                QGCFileDialog {
                    id:                 savePathBrowseDialog
                    title:              qsTr("Choose the location to save/load files")
                    folder:             _appSavePath.rawValue
                    selectFolder:       true
                    onAcceptedForLoad:  (file) => _appSavePath.rawValue = file
                }
            }
        }

        FactCheckBoxSlider {
            Layout.fillWidth:   true
            text:               qsTr("Save application data to SD Card")
            fact:               _androidSaveToSDCard
            visible:            _androidSaveToSDCard.visible
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true
        heading:            qsTr("Units")
        visible:            _unitsSettings.visible

        LabelledComboBox {
            label:          qsTr("Unit system")
            model:          [ qsTr("Metric"), qsTr("Imperial"), qsTr("Custom") ]
            currentIndex:   _unitSystemIndex
            onActivated:    (index) => _unitsSettings.setUnitSystem(index)
        }

        Repeater {
            model: _unitSystemIndex === UnitsSettings.UnitSystemCustom
                       ? [ _unitsSettings.horizontalDistanceUnits, _unitsSettings.verticalDistanceUnits,
                           _unitsSettings.areaUnits, _unitsSettings.speedUnits, _unitsSettings.temperatureUnits ]
                       : []

            LabelledFactComboBox {
                label:      modelData.shortDescription
                fact:       modelData
                indexModel: false
            }
        }
    }

    SettingsGroupLayout {
        Layout.fillWidth:   true

        LabelledButton {
            label:      _resetPending ? qsTr("All settings will be cleared on next start") : ""
            buttonText: _resetPending ? qsTr("Cancel Reset") : qsTr("Reset All Settings…")
            onClicked:  {
                if (_resetPending) {
                    QGroundControl.clearDeleteAllSettingsNextBoot()
                    _resetPending = false
                } else {
                    mainWindow.showMessageDialog(
                        qsTr("Reset All Settings"),
                        qsTr("All settings will be cleared the next time %1 starts. This cannot be undone.").arg(QGroundControl.appName),
                        Dialog.Ok | Dialog.Cancel,
                        function() {
                            QGroundControl.deleteAllSettingsNextBoot()
                            _resetPending = true
                        })
                }
            }
        }
    }
}
