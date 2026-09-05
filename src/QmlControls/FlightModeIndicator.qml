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

    readonly property string _flightMode:   activeVehicle ? activeVehicle.flightMode : ""
    readonly property bool   _connecting:   activeVehicle && _flightMode === ""
    readonly property bool   _modeKnown:    activeVehicle && activeVehicle.flightModes.indexOf(_flightMode) >= 0

    readonly property var _modeGlyphs: [
        { keywords: [ "return", "rtl", "land", "smart" ],                         icon: "/InstrumentValueIcons/home.svg" },
        { keywords: [ "mission", "auto", "takeoff", "offboard", "follow" ],       icon: "/InstrumentValueIcons/play.svg" },
        { keywords: [ "position", "loiter", "hold", "guided", "brake", "orbit" ], icon: "/InstrumentValueIcons/target.svg" },
        { keywords: [ "manual", "stabil", "acro", "altitude", "alt", "sport" ],   icon: "/InstrumentValueIcons/hand-stop.svg" },
    ]

    function _modeIcon(mode) {
        const name = mode.toLowerCase()
        const match = _modeGlyphs.find((glyph) => glyph.keywords.some((keyword) => name.indexOf(keyword) !== -1))
        return match ? match.icon : "/qmlimages/FlightModesComponentIcon.png"
    }

    RowLayout {
        Layout.fillWidth:   true
        spacing:            ScreenTools.defaultFontPixelWidth

        QGCColoredImage {
            id:                 flightModeIcon
            width:              ScreenTools.defaultFontPixelHeight
            height:             ScreenTools.defaultFontPixelHeight
            sourceSize.height:  height
            fillMode:           Image.PreserveAspectFit
            mipmap:             true
            color:              qgcPal.toolbarText
            source:             control._modeIcon(control._flightMode)
            visible:            !control._connecting
        }

        QGCLabel {
            text:               !activeVehicle ? qsTr("N/A", "No data to display")
                                : control._connecting ? qsTr("Connecting…")
                                : control._flightMode
            color:              control._connecting ? qgcPal.colorGrey : qgcPal.toolbarText
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

            QGCLabel {
                Layout.fillWidth:       true
                Layout.maximumWidth:    ScreenTools.defaultFontPixelWidth * 36
                wrapMode:               Text.WordWrap
                font.pointSize:         ScreenTools.smallFontPointSize
                color:                  QGroundControl.globalPalette.colorOrange
                visible:                modeColumn.activeVehicle && !control._connecting && !control._modeKnown
                text:                   qsTr("The vehicle is in %1, which this version of the app doesn't know. Choose a mode below to change it.").arg(control._flightMode)
            }

            ModeSection { model: modeColumn._flightModes }

            OverlayMenuSeparator { visible: modeColumn._returnModes.length > 0 && modeColumn._flightModes.length > 0 }

            ModeSection { model: modeColumn._returnModes }

            OverlayMenuSeparator { visible: modeColumn._devModes.length > 0 && modeColumn._allModes.length > modeColumn._devModes.length }

            ModeSection { model: modeColumn._devModes }

            OverlayMenuSeparator { visible: showAllModesItem.visible }

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
                popoverStyle: true
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
