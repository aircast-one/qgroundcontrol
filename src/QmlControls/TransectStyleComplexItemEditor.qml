
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.FlightMap

Item {
    id:     _root
    height: editorColumn.height
    width:  availableWidth

    property bool   transectAreaDefinitionComplete: true
    property string transectAreaDefinitionHelp:     _internalError
    property string transectValuesHeaderName:       _internalError
    property var    transectValuesComponent:        undefined
    property var    presetsTransectValuesComponent: undefined
    property string entryPointText:                 qsTr("Rotate entry point")
    property string entryPointValue:                ""

    readonly property string _internalError: "Internal Error"

    property var    _missionItem:               missionItem
    property real   _cameraMinTriggerInterval:  _missionItem.cameraCalc.minTriggerInterval.rawValue
    property bool   _presetsAvailable:          _missionItem.presetNames.length !== 0

    readonly property real _gap: ScreenTools.defaultFontPixelHeight * 0.7

    function polygonCaptureStarted() {
        _missionItem.clearPolygon()
    }

    function polygonCaptureFinished(coordinates) {
        for (const coordinate of coordinates) {
            _missionItem.addPolygonCoordinate(coordinate)
        }
    }

    function polygonAdjustVertex(vertexIndex, vertexCoordinate) {
        _missionItem.adjustPolygonCoordinate(vertexIndex, vertexCoordinate)
    }

    function polygonAdjustStarted() { }
    function polygonAdjustFinished() { }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    Column {
        id:            editorColumn
        anchors.left:  parent.left
        anchors.right: parent.right
        spacing:       _root._gap

        PlanGroupCard {
            width:   parent.width
            visible: !transectAreaDefinitionComplete || _missionItem.wizardMode

            Item {
                width:  parent.width
                height: helpLabel.implicitHeight + ScreenTools.defaultFontPixelHeight

                QGCLabel {
                    id:                     helpLabel
                    anchors.left:           parent.left
                    anchors.right:          parent.right
                    anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    wrapMode:               Text.WordWrap
                    color:                  Qt.alpha(qgcPal.text, 0.7)
                    text:                   transectAreaDefinitionHelp
                }
            }
        }

        Column {
            width:   parent.width
            spacing: _root._gap
            visible: transectAreaDefinitionComplete && !_missionItem.wizardMode

            TransectStyleComplexItemTabBar {
                id:    tabBar
                width: parent.width
            }

            Column {
                width:   parent.width
                spacing: _root._gap
                visible: tabBar.currentIndex === 0

                PlanGroupCard {
                    width:   parent.width
                    visible: _missionItem.cameraShots > 0 && _cameraMinTriggerInterval !== 0 && _cameraMinTriggerInterval > _missionItem.timeBetweenShots

                    PlanGroupRow {
                        text:        qsTr("Photo interval too short")
                        description: qsTr("The camera needs at least %1 s between photos").arg(_cameraMinTriggerInterval.toFixed(1))
                        textColor:   qgcPal.warningText
                    }
                }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        objectName:  "transectCameraRow"
                        text:        qsTr("Camera")
                        value:       _missionItem.cameraCalc.isManualCamera ? qsTr("Manual")
                                   : _missionItem.cameraCalc.isCustomCamera ? qsTr("Custom")
                                                                            : _missionItem.cameraCalc.cameraBrand + " " + _missionItem.cameraCalc.cameraModel
                        showChevron: true
                        interactive: true
                        onClicked:   tabBar.currentIndex = 1
                    }
                }

                CameraCalcGrid {
                    width:                  parent.width
                    cameraCalc:             _missionItem.cameraCalc
                    distanceToSurfaceLabel: qsTr("Altitude")
                    frontalDistanceLabel:   qsTr("Trigger distance")
                    sideDistanceLabel:      qsTr("Spacing")
                }

                PlanSectionLabel { text: transectValuesHeaderName.toUpperCase() }

                Loader {
                    width:           parent.width
                    sourceComponent: transectValuesComponent

                    property bool forPresets: false
                }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        text:        entryPointText
                        value:       entryPointValue
                        showChevron: true
                        interactive: true
                        onClicked:   _missionItem.rotateEntryPoint()
                    }
                }

                PlanSectionLabel { text: qsTr("STATISTICS") }

                TransectStyleComplexItemStats { width: parent.width }
            }

            CameraCalcCamera {
                width:      parent.width
                visible:    tabBar.currentIndex === 1
                cameraCalc: _missionItem.cameraCalc
            }

            TransectStyleComplexItemTerrainFollow {
                width:       parent.width
                spacing:     ScreenTools.defaultFontPixelWidth / 2
                visible:     tabBar.currentIndex === 2
                missionItem: _missionItem
            }

            Column {
                width:   parent.width
                spacing: _root._gap
                visible: tabBar.currentIndex === 3

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        text: qsTr("Preset")

                        QGCComboBox {
                            id:                     presetCombo
                            anchors.verticalCenter: parent.verticalCenter
                            width:                  ScreenTools.defaultFontPixelWidth * 20
                            model:                  _missionItem.presetNames
                        }
                    }

                    PlanGroupRow {
                        text:        qsTr("Apply preset")
                        textColor:   qgcPal.primaryButton
                        interactive: true
                        enabled:     _presetsAvailable
                        onClicked:   _missionItem.loadPreset(presetCombo.textAt(presetCombo.currentIndex))
                    }

                    PlanGroupRow {
                        text:        qsTr("Delete preset")
                        textColor:   qgcPal.colorRed
                        interactive: true
                        enabled:     _presetsAvailable
                        onClicked:   deletePresetDialog.createObject(mainWindow, { presetName: presetCombo.textAt(presetCombo.currentIndex) }).open()
                    }
                }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        text:        qsTr("Save settings as new preset")
                        textColor:   qgcPal.primaryButton
                        interactive: true
                        onClicked:   savePresetDialog.createObject(mainWindow).open()
                    }
                }

                PlanSectionLabel {
                    text:    transectValuesHeaderName.toUpperCase()
                    visible: !!presetsTransectValuesComponent
                }

                Loader {
                    width:           parent.width
                    visible:         !!presetsTransectValuesComponent
                    sourceComponent: presetsTransectValuesComponent

                    property bool forPresets: true
                }

                PlanSectionLabel { text: qsTr("STATISTICS") }

                TransectStyleComplexItemStats { width: parent.width }
            }
        }
    }

    Component {
        id: deletePresetDialog

        QGCSimpleMessageDialog {
            title:      qsTr("Delete Preset")
            text:       qsTr("Are you sure you want to delete '%1' preset?").arg(presetName)
            buttons:    Dialog.Yes | Dialog.No

            property string presetName

            onAccepted: { _missionItem.deletePreset(presetName) }
        }
    }

    Component {
        id: savePresetDialog

        QGCPopupDialog {
            id:         popupDialog
            title:      qsTr("Save Preset")
            buttons:    Dialog.Save | Dialog.Cancel

            onAccepted: {
                if (presetNameField.text != "") {
                    _missionItem.savePreset(presetNameField.text.trim())
                } else {
                    preventClose = true
                }
            }

            ColumnLayout {
                width:      ScreenTools.defaultFontPixelWidth * 30
                spacing:    ScreenTools.defaultFontPixelHeight

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               qsTr("Save the current settings as a named preset.")
                    wrapMode:           Text.WordWrap
                }

                QGCLabel {
                    text: qsTr("Preset Name")
                }

                QGCTextField {
                    id:                 presetNameField
                    Layout.fillWidth:   true
                    placeholderText:    qsTr("Enter preset name")

                    Component.onCompleted:  validateText(presetNameField.text)
                    onTextChanged:          validateText(text)

                    function validateText(text) {
                        if (text.trim() === "") {
                            nameError.text = qsTr("Preset name cannot be blank.")
                            popupDialog.acceptButtonEnabled = false
                        } else if (text.includes("/")) {
                            nameError.text = qsTr("Preset name cannot include the \"/\" character.")
                            popupDialog.acceptButtonEnabled = false
                        } else {
                            nameError.text = ""
                            popupDialog.acceptButtonEnabled = true
                        }
                    }
                }

                QGCLabel {
                    id:                 nameError
                    Layout.fillWidth:   true
                    wrapMode:           Text.WordWrap
                    color:              QGroundControl.globalPalette.warningText
                    visible:            text !== ""
                }
            }
        }
    }
}
