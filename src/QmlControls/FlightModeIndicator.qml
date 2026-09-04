/****************************************************************************
 *
 * (c) 2009-2022 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
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
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.AutoPilotPlugin

RowLayout {
    id:         control
    spacing:    0

    property bool   showIndicator:          true
    property var    expandedPageComponent
    property bool   waitForParameters:      false

    property real fontPointSize:    ScreenTools.largeFontPointSize
    property var  activeVehicle:    QGroundControl.multiVehicleManager.activeVehicle
    property bool allowEditMode:    true
    property bool editMode:         false

    RowLayout {
        Layout.fillWidth: true

        QGCColoredImage {
            id:         flightModeIcon
            width:      ScreenTools.defaultFontPixelWidth * 3
            height:     ScreenTools.defaultFontPixelHeight
            fillMode:   Image.PreserveAspectFit
            mipmap:     true
            color:      qgcPal.toolbarText
            source:     "/qmlimages/FlightModesComponentIcon.png"
        }

        QGCLabel {
            text:               activeVehicle ? activeVehicle.flightMode : qsTr("N/A", "No data to display")
            color:              qgcPal.toolbarText
            font.pointSize:     fontPointSize
            Layout.alignment:   Qt.AlignCenter

            MouseArea {
                anchors.fill:   parent
                onClicked:      mainWindow.showIndicatorDrawer(drawerComponent, control)
            }
        }
    }

    Component {
        id: drawerComponent

        ToolIndicatorPage {
            showExpand:         true
            waitForParameters:  control.waitForParameters

            contentComponent:    flightModeContentComponent
            expandedComponent:   flightModeExpandedComponent

            onExpandedChanged: {
                if (!expanded) {
                    editMode = false
                }
            }
        }
    }

    Component {
        id: flightModeContentComponent

        ColumnLayout {
            id:         modeColumn
            spacing:    ScreenTools.defaultFontPixelWidth / 4

            property var    activeVehicle:            QGroundControl.multiVehicleManager.activeVehicle
            property var    flightModeSettings:       QGroundControl.settingsManager.flightModeSettings
            property var    hiddenFlightModesFact:    null
            property var    hiddenFlightModesList:    []

            readonly property var _allModes:    activeVehicle ? activeVehicle.flightModes : []
            readonly property var _returnModes: _allModes.filter((mode) => _isReturnMode(mode))
            readonly property var _devModes:    _allModes.filter((mode) => _isDevMode(mode))
            readonly property var _flightModes: _allModes.filter((mode) => !_isReturnMode(mode) && !_isDevMode(mode))

            // Firmwares name their modes freely, so grouping is by name and anything
            // unrecognised stays in the main group rather than disappearing from the menu.
            readonly property var _returnKeywords: [ "return", "rtl", "land" ]

            function _isReturnMode(mode) {
                const name = mode.toLowerCase()
                return _returnKeywords.some((keyword) => name.indexOf(keyword) !== -1)
            }

            function _isDevMode(mode) {
                return mode.toLowerCase().indexOf("mocklink") !== -1
            }

            function _isHidden(mode) {
                return hiddenFlightModesList.indexOf(mode) !== -1
            }

            function _setHidden(mode, hidden) {
                const remaining = hiddenFlightModesList.filter((item) => item !== mode)
                hiddenFlightModesList = hidden ? [ ...remaining, mode ] : remaining
                hiddenFlightModesFact.value = hiddenFlightModesList.join(",")
            }

            function _showAllModes() {
                hiddenFlightModesList = []
                hiddenFlightModesFact.value = ""
            }

            Component.onCompleted: {
                // Hidden flight modes are classified by firmware and vehicle class
                var hiddenFlightModesPropPrefix
                if (activeVehicle.px4Firmware) {
                    hiddenFlightModesPropPrefix = "px4HiddenFlightModes"
                } else if (activeVehicle.apmFirmware) {
                    hiddenFlightModesPropPrefix = "apmHiddenFlightModes"
                } else {
                    control.allowEditMode = false
                }
                if (control.allowEditMode) {
                    var hiddenFlightModesProp = hiddenFlightModesPropPrefix + activeVehicle.vehicleClassInternalName()
                    if (flightModeSettings.hasOwnProperty(hiddenFlightModesProp)) {
                        hiddenFlightModesFact = flightModeSettings[hiddenFlightModesProp]
                        // Split string into list of flight modes
                        if (hiddenFlightModesFact && hiddenFlightModesFact.value !== "") {
                            hiddenFlightModesList = hiddenFlightModesFact.value.split(",")
                        }
                    } else {
                        control.allowEditMode = false
                    }
                }
            }

            component ModeSection: Repeater {
                delegate: RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth
                    visible:            control.editMode || !modeColumn._isHidden(modelData)

                    OverlayMenuItem {
                        text:       modelData
                        checkable:  true
                        checked:    modeColumn.activeVehicle && modeColumn.activeVehicle.flightMode === modelData

                        onClicked: {
                            if (control.editMode) {
                                modeColumn._setHidden(modelData, !modeColumn._isHidden(modelData))
                            } else {
                                modeColumn.activeVehicle.flightMode = modelData
                                mainWindow.closeIndicatorDrawer()
                            }
                        }
                    }

                    QGCCheckBoxSlider {
                        visible:    control.editMode
                        checked:    !modeColumn._isHidden(modelData)
                        onClicked:  modeColumn._setHidden(modelData, !checked)
                    }
                }
            }

            ModeSection { model: modeColumn._flightModes }

            OverlayMenuSeparator { visible: modeColumn._returnModes.length > 0 && modeColumn._flightModes.length > 0 }

            ModeSection { model: modeColumn._returnModes }

            OverlayMenuSeparator { visible: modeColumn._devModes.length > 0 && modeColumn._allModes.length > modeColumn._devModes.length }

            ModeSection { model: modeColumn._devModes }

            OverlayMenuSeparator { visible: showAllModesItem.visible }

            // A count of what is missing is a dead end; the row that reveals them is not.
            OverlayMenuItem {
                id:         showAllModesItem
                text:       qsTr("Show All Modes")
                visible:    modeColumn.hiddenFlightModesList.length > 0 && !control.editMode
                onClicked:  modeColumn._showAllModes()
            }
        }
    }

    Component {
        id: flightModeExpandedComponent

        ColumnLayout {
            Layout.preferredWidth:  ScreenTools.defaultFontPixelWidth * 60
            spacing:                margins / 2

            property var  qgcPal:   QGroundControl.globalPalette
            property real margins:  ScreenTools.defaultFontPixelHeight

            Loader {
                sourceComponent: expandedPageComponent
            }

            SettingsGroupLayout {
                Layout.fillWidth:  true

                RowLayout {
                    Layout.fillWidth:   true
                    enabled:            control.allowEditMode

                    QGCLabel {
                        Layout.fillWidth:   true
                        text:               qsTr("Edit Displayed Flight Modes")
                    }

                    QGCCheckBoxSlider {
                        onClicked: control.editMode = checked
                    }
                }

                LabelledButton {
                    Layout.fillWidth:   true
                    label:              qsTr("Flight Modes")
                    buttonText:         qsTr("Configure")
                    visible:            _activeVehicle.autopilotPlugin.knownVehicleComponentAvailable(AutoPilotPlugin.KnownFlightModesVehicleComponent) &&
                                            QGroundControl.corePlugin.showAdvancedUI

                    onClicked: {
                        mainWindow.showKnownVehicleComponentConfigPage(AutoPilotPlugin.KnownFlightModesVehicleComponent)
                        mainWindow.closeIndicatorDrawer()
                    }
                }
            }
        }
    }
}
