import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Vehicle
import QGroundControl.Controls
import QGroundControl.FactControls
import QGroundControl.Palette

Item {
    id:     _root
    width:  availableWidth
    height: editorColumn.implicitHeight

    property bool _specifiesAltitude:       missionItem.specifiesAltitude
    property real _margin:                  ScreenTools.defaultFontPixelHeight / 2
    property real _altRectMargin:           ScreenTools.defaultFontPixelWidth / 2
    property var  _controllerVehicle:       missionItem.masterController.controllerVehicle
    property int  _globalAltMode:           missionItem.masterController.missionController.globalAltitudeMode
    property bool _globalAltModeIsMixed:    _globalAltMode == QGroundControl.AltitudeModeMixed
    property real _radius:                  ScreenTools.defaultFontPixelWidth / 2
    property real _fieldWidth:              ScreenTools.defaultFontPixelWidth * 12

    readonly property string _altModeText: QGroundControl.altitudeModeShortDescription(missionItem.altitudeMode)

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    ColumnLayout {
        id:                 editorColumn
        anchors.left:       parent.left
        anchors.right:      parent.right
        anchors.top:        parent.top
        spacing:            _margin

        ColumnLayout {
            Layout.fillWidth:   true
            spacing:            _margin
            visible:            missionItem.isTakeoffItem && missionItem.wizardMode

            QGCLabel {
                Layout.fillWidth:   true
                wrapMode:           Text.WordWrap
                visible:            !initialClickLabel.visible
                text:               _controllerVehicle.vtol
                                        ? qsTr("Move 'T' Transition Direction to the desired location. Ensure distance from launch to transition direction is far enough to complete transition.")
                                        : qsTr("Move 'T' Takeoff to the climbout location.")
            }

            QGCLabel {
                Layout.fillWidth:   true
                wrapMode:           Text.WordWrap
                color:              qgcPal.colorGrey
                font.pointSize:     ScreenTools.smallFontPointSize
                text:               qsTr("Ensure clear of obstacles and into the wind.")
                visible:            !initialClickLabel.visible
            }

            QGCButton {
                Layout.fillWidth:   true
                text:               qsTr("Done")
                visible:            !initialClickLabel.visible
                onClicked:          missionItem.wizardMode = false
            }

            QGCLabel {
                id:                 initialClickLabel
                Layout.fillWidth:   true
                wrapMode:           Text.WordWrap
                text:               missionItem.launchTakeoffAtSameLocation ?
                                        qsTr("Click in map to set planned Takeoff location.") :
                                        qsTr("Click in map to set planned Launch location.")
                visible:            missionItem.isTakeoffItem && !missionItem.launchCoordinate.isValid
            }
        }

        SettingsGroupLayout {
            Layout.fillWidth:   true
            popoverStyle:       true
            cardStyle:          true
            visible:            !missionItem.wizardMode && _specifiesAltitude
            description:        missionItem.isLandCommand
                                    ? qsTr("Altitude is the approximate ground altitude. Normally 0 when landing back at the launch location.")
                                    : (missionItem.altitudeMode === QGroundControl.AltitudeModeCalcAboveTerrain
                                        ? qsTr("Actual AMSL alt sent: %1 %2").arg(missionItem.amslAltAboveTerrain.valueString).arg(missionItem.amslAltAboveTerrain.units)
                                        : "")

            LabelledFactTextField {
                Layout.fillWidth:           true
                label:                      qsTr("Altitude")
                fact:                       missionItem.altitude
                textFieldPreferredWidth:    _fieldWidth
            }

            RowLayout {
                id:                     altModeRow
                Layout.fillWidth:       true
                Layout.preferredHeight: ScreenTools.settingsRowHeight
                spacing:                ScreenTools.defaultFontPixelWidth * 2
                visible:                _globalAltMode !== QGroundControl.AltitudeModeRelative

                QGCLabel {
                    Layout.fillWidth:   true
                    Layout.alignment:   Qt.AlignVCenter
                    text:               qsTr("Reference")
                }

                QGCLabel {
                    id:                 altModeLabel
                    Layout.alignment:   Qt.AlignVCenter
                    text:               _root._altModeText
                    color:              qgcPal.colorGrey
                }

                QGCColoredImage {
                    Layout.alignment:   Qt.AlignVCenter
                    height:             ScreenTools.defaultFontPixelHeight / 2
                    width:              height
                    source:             "/res/DropArrow.svg"
                    color:              altModeLabel.color
                    visible:            _globalAltModeIsMixed
                }

                TapHandler {
                    enabled:  _globalAltModeIsMixed
                    onTapped: altModeMenu.openFrom(altModeRow)
                }

                AltModeMenu {
                    id:              altModeMenu
                    objectName:      "itemAltModeMenu"
                    currentAltMode:  missionItem.altitudeMode
                    rgRemoveModes:   _controllerVehicle.supportsTerrainFrame
                                        ? [QGroundControl.AltitudeModeMixed]
                                        : [QGroundControl.AltitudeModeMixed, QGroundControl.AltitudeModeTerrainFrame]
                    updateAltModeFn: (altMode) => { missionItem.altitudeMode = altMode }
                }
            }
        }

        SettingsGroupLayout {
            Layout.fillWidth:   true
            popoverStyle:       true
            cardStyle:          true
            visible:            !missionItem.wizardMode

            Repeater {
                model: missionItem.comboboxFacts

                LabelledFactMenu {
                    Layout.fillWidth:   true
                    label:              object.name
                    fact:               object
                }
            }

            Repeater {
                model: missionItem.textFieldFacts

                LabelledFactTextField {
                    Layout.fillWidth:           true
                    label:                      object.name
                    fact:                       object
                    textFieldShowUnits:         true
                    textFieldPreferredWidth:    _fieldWidth
                    textField.enabled:          !object.readOnly
                }
            }

            Repeater {
                model: missionItem.nanFacts

                RowLayout {
                    Layout.fillWidth:       true
                    Layout.preferredHeight: ScreenTools.settingsRowHeight
                    spacing:                ScreenTools.defaultFontPixelWidth * 2

                    QGCCheckBoxSlider {
                        Layout.fillWidth:   true
                        Layout.alignment:   Qt.AlignVCenter
                        text:               object.name
                        checked:            !isNaN(object.rawValue)
                        onClicked:          object.rawValue = checked ? 0 : NaN
                    }

                    FactTextField {
                        Layout.preferredWidth:  _fieldWidth
                        Layout.alignment:       Qt.AlignVCenter
                        fact:                   object
                        showUnits:              true
                        enabled:                !isNaN(object.rawValue)
                        showFrame:              false
                        horizontalAlignment:    TextInput.AlignRight
                    }
                }
            }

            RowLayout {
                Layout.fillWidth:       true
                Layout.preferredHeight: ScreenTools.settingsRowHeight
                spacing:                ScreenTools.defaultFontPixelWidth * 2
                visible:                missionItem.speedSection.available

                QGCCheckBoxSlider {
                    id:                 flightSpeedCheckbox
                    Layout.fillWidth:   true
                    Layout.alignment:   Qt.AlignVCenter
                    text:               qsTr("Flight Speed")
                    checked:            missionItem.speedSection.specifyFlightSpeed
                    onClicked:          missionItem.speedSection.specifyFlightSpeed = checked
                }

                FactTextField {
                    Layout.preferredWidth:  _fieldWidth
                    Layout.alignment:       Qt.AlignVCenter
                    fact:                   missionItem.speedSection.flightSpeed
                    enabled:                flightSpeedCheckbox.checked
                    showFrame:              false
                    horizontalAlignment:    TextInput.AlignRight
                }
            }
        }

        CameraSection {
            Layout.fillWidth:   true
            checked:            missionItem.cameraSection.settingsSpecified
            visible:            missionItem.cameraSection.available && !missionItem.wizardMode
        }

        // What the command does, read as a footnote under the group it explains - the place
        // every other explanatory line in this panel sits. Above the fields it was a paragraph
        // wedged between the selected row and its own form.
        QGCLabel {
            Layout.fillWidth:   true
            wrapMode:           Text.WordWrap
            font.pointSize:     ScreenTools.smallFontPointSize
            color:              qgcPal.colorGrey
            text:               missionItem.rawEdit ?
                                    qsTr("Provides advanced access to all commands/parameters. Be very careful!") :
                                    missionItem.commandDescription
            visible:            text !== ""
        }
    }
}
