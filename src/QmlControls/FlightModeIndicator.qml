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
    objectName: "flightModeControl"
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
    readonly property bool   _canSetMode:   activeVehicle && activeVehicle.flightModeSetAvailable

    property string          _pendingMode:  ""
    property string          _rejection:    ""
    readonly property bool   _pending:      _pendingMode !== ""
    readonly property bool   _rejected:     _rejection !== ""
    readonly property string _shownMode:    _rejected ? _rejection : _pending ? _pendingMode : _flightMode

    readonly property int _mavCmdDoSetMode:   176
    readonly property int _mavResultAccepted: 0

    readonly property var _rejectionWording: ({
        1: qsTr("%1 refused for now"),
        2: qsTr("%1 denied"),
        3: qsTr("%1 not supported"),
    })

    readonly property var _modeGlyphs: [
        { keywords: [ "return", "rtl", "land", "smart", "surface", "dock" ],             icon: "/InstrumentValueIcons/home.svg" },
        { keywords: [ "mission", "auto", "takeoff", "offboard", "follow", "zigzag" ],    icon: "/InstrumentValueIcons/play.svg" },
        { keywords: [ "position", "loiter", "hold", "guided", "brake", "orbit", "circle" ], icon: "/InstrumentValueIcons/target.svg" },
        { keywords: [ "manual", "stabil", "acro", "altitude", "alt", "sport", "fbw", "cruise", "steering", "rattitude", "training", "drift", "hover" ], icon: "/InstrumentValueIcons/joystick.svg" },
    ]

    readonly property var _modeDescriptions: ({
        "Stabilize":                qsTr("You fly it by hand, it only levels itself"),
        "Stabilized":               qsTr("You fly it by hand, it only levels itself"),
        "Manual":                   qsTr("Sticks go straight to the motors, no help"),
        "Acro":                     qsTr("Sticks set rotation rate, no self-levelling"),
        "Altitude Hold":            qsTr("Holds height, you steer"),
        "Altitude":                 qsTr("Holds height, you steer"),
        "Depth Hold":               qsTr("Holds depth, you steer"),
        "Position Hold":            qsTr("Holds position and height, sticks nudge it"),
        "Position":                 qsTr("Holds position and height, sticks nudge it"),
        "Loiter":                   qsTr("Holds position and height, or circles it on a plane"),
        "Hold":                     qsTr("Stops and holds where it is"),
        "Brake":                    qsTr("Stops as fast as it can and holds"),
        "Guided":                   qsTr("Flies to points you tap on the map"),
        "Guided No GPS":            qsTr("Accepts attitude commands without a position fix"),
        "Auto":                     qsTr("Flies the uploaded mission"),
        "Mission":                  qsTr("Flies the uploaded mission"),
        "RTL":                      qsTr("Climbs, returns home and lands"),
        "Return":                   qsTr("Climbs, returns home and lands"),
        "Smart RTL":                qsTr("Retraces its own path back home"),
        "AutoRTL":                  qsTr("Follows the mission's landing sequence home"),
        "Return to Groundstation":  qsTr("Returns to the ground station"),
        "Land":                     qsTr("Lands straight down where it is"),
        "Precision Land":           qsTr("Lands on the landing target"),
        "Precision Landing":        qsTr("Lands on the landing target"),
        "Takeoff":                  qsTr("Climbs to takeoff height and holds"),
        "Circle":                   qsTr("Circles the point below it"),
        "Orbit":                    qsTr("Circles a point you choose"),
        "Follow":                   qsTr("Follows the ground station or a beacon"),
        "Follow Me":                qsTr("Follows the ground station"),
        "Drift":                    qsTr("Coordinated turns, like a plane"),
        "Sport":                    qsTr("Rate control with height hold"),
        "Flip":                     qsTr("Does one flip, then returns to the previous mode"),
        "Throw":                    qsTr("Starts flying when thrown"),
        "Autotune":                 qsTr("Tunes the controllers automatically, needs room"),
        "Flow Hold":                qsTr("Holds position with optical flow, no GPS"),
        "ZigZag":                   qsTr("Sweeps between two points you record"),
        "SystemID":                 qsTr("Injects test signals for system identification"),
        "AutoRotate":               qsTr("Helicopter autorotation after engine loss"),
        "Avoid ADSB":               qsTr("Dodges ADS-B traffic automatically"),
        "Turtle":                   qsTr("Flips itself upright after a crash"),
        "Cruise":                   qsTr("Holds heading and height, sticks trim"),
        "FBW A":                    qsTr("Sticks set bank and pitch, wings stay level"),
        "FBW B":                    qsTr("Sticks set height and heading"),
        "Training":                 qsTr("Manual with bank and pitch limits"),
        "Thermal":                  qsTr("Circles rising air automatically"),
        "Autoland":                 qsTr("Lands on the runway automatically"),
        "Loiter to QLand":          qsTr("Circles, then lands as a quadcopter"),
        "QuadPlane Stabilize":      qsTr("Hovers by hand, it only levels itself"),
        "QuadPlane Hover":          qsTr("Hovers holding height, you steer"),
        "QuadPlane Loiter":         qsTr("Hovers holding position and height"),
        "QuadPlane Land":           qsTr("Lands as a quadcopter where it is"),
        "QuadPlane RTL":            qsTr("Returns home and lands as a quadcopter"),
        "QuadPlane AutoTune":       qsTr("Tunes the hover controllers automatically"),
        "QuadPlane Acro":           qsTr("Rate control while hovering"),
        "Steering":                 qsTr("Sticks set speed and turn rate"),
        "Learning":                 qsTr("Records waypoints as you drive"),
        "Simple":                   qsTr("Sticks steer relative to where you stand"),
        "Dock":                     qsTr("Drives onto the docking target"),
        "Surface":                  qsTr("Rises to the surface"),
        "Surftrak":                 qsTr("Holds a set distance above the seabed"),
        "Motor Detection":          qsTr("Works out motor order and direction"),
        "Rattitude":                qsTr("Levels near centre, rate control at full stick"),
        "Offboard":                 qsTr("Controlled by a companion computer"),
        "Ready":                    qsTr("Armed and waiting on the ground"),
        "Initializing":             qsTr("Booting, cannot fly yet"),
    })

    function _modeIcon(mode) {
        const name = mode.toLowerCase()
        const match = _modeGlyphs.find((glyph) => glyph.keywords.some((keyword) => name.indexOf(keyword) !== -1))
        return match ? match.icon : "/qmlimages/FlightModesComponentIcon.png"
    }

    function _modeDescription(mode) {
        return _modeDescriptions[mode] || ""
    }

    function _requestMode(mode) {
        if (mode !== _flightMode) {
            _pendingMode = mode
            pendingTimer.restart()
        }
        activeVehicle.flightMode = mode
    }

    function _settlePending() {
        _pendingMode = ""
        pendingTimer.stop()
    }

    function _reject(wording) {
        _rejection = wording.arg(_pendingMode)
        _settlePending()
        rejectionTimer.restart()
    }

    Timer {
        id:          pendingTimer
        interval:    3000
        onTriggered: control._reject(qsTr("%1: no reply"))
    }

    Timer {
        id:          rejectionTimer
        interval:    2500
        onTriggered: control._rejection = ""
    }

    Connections {
        target: control.activeVehicle

        function onFlightModeChanged() {
            control._settlePending()
            control._rejection = ""
        }

        function onMavCommandResult(vehicleId, targetComponent, command, ackResult, failureCode) {
            if (control._pending && command === control._mavCmdDoSetMode && ackResult !== control._mavResultAccepted) {
                control._reject(control._rejectionWording[ackResult] || qsTr("%1 failed"))
            }
        }
    }

    Item {
        Layout.fillWidth:   true
        Layout.fillHeight:  true
        implicitWidth:      indicatorRow.implicitWidth
        implicitHeight:     Math.max(indicatorRow.implicitHeight, ScreenTools.defaultFontPixelHeight * 2.5)

        RowLayout {
            id:             indicatorRow
            anchors.fill:   parent
            spacing:        ScreenTools.defaultFontPixelWidth

            QGCColoredImage {
                width:              ScreenTools.defaultFontPixelHeight
                height:             ScreenTools.defaultFontPixelHeight
                sourceSize.height:  height
                fillMode:           Image.PreserveAspectFit
                mipmap:             true
                color:              qgcPal.toolbarText
                source:             control._modeIcon(control._shownMode)
                visible:            !control._connecting && !control._pending
            }

            QGCSpinner {
                objectName: "flightModePending"
                width:      ScreenTools.defaultFontPixelHeight
                height:     width
                color:      qgcPal.toolbarText
                visible:    control._pending
            }

            QGCLabel {
                objectName:         "flightModeLabel"
                text:               !activeVehicle ? qsTr("N/A", "No data to display")
                                    : control._connecting ? qsTr("Connecting…")
                                    : control._shownMode
                color:              control._connecting ? qgcPal.colorGrey
                                    : control._rejected ? qgcPal.colorOrange
                                    : qgcPal.toolbarText
                opacity:            control._pending ? 0.6 : 1
                font.pointSize:     fontPointSize
                Layout.alignment:   Qt.AlignCenter
            }

            QGCColoredImage {
                width:              ScreenTools.defaultFontPixelHeight * 0.6
                height:             width
                sourceSize.height:  height
                fillMode:           Image.PreserveAspectFit
                mipmap:             true
                color:              qgcPal.toolbarText
                opacity:            0.7
                source:             "/InstrumentValueIcons/cheveron-down.svg"
                visible:            control._canSetMode && !control._connecting
            }
        }

        MouseArea {
            anchors.fill:   parent
            enabled:        control._canSetMode
            onClicked:      mainWindow.showIndicatorDrawer(drawerComponent, control)
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
            objectName: "flightModeList"
            spacing:    ScreenTools.defaultFontPixelWidth / 4

            property var    activeVehicle:            QGroundControl.multiVehicleManager.activeVehicle
            property var    flightModeSettings:       QGroundControl.settingsManager.flightModeSettings
            property var    hiddenFlightModesFact:    null
            property var    hiddenFlightModesList:    []
            property bool   showAdvanced:             false

            readonly property var _allModes:      activeVehicle ? activeVehicle.flightModes : []
            readonly property var _advancedModes: activeVehicle ? activeVehicle.advancedFlightModes : []
            readonly property var _returnModes:   _allModes.filter((mode) => _isReturnMode(mode))
            readonly property var _devModes:      _allModes.filter((mode) => _isDevMode(mode))
            readonly property var _flightModes:   _allModes.filter((mode) => !_isReturnMode(mode) && !_isDevMode(mode))
            readonly property bool _moreAvailable: !showAdvanced && !control.editMode
                                                   && _allModes.some((mode) => (_isAdvanced(mode) || _isHidden(mode)) && mode !== control._flightMode)
            readonly property int  _listedCount:   _allModes.filter((mode) => _isListed(mode)).length
            readonly property bool _compact:       _listedCount * ScreenTools.defaultFontPixelHeight * 4.2 > mainWindow.height * 0.85

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

            function _isAdvanced(mode) {
                return _advancedModes.indexOf(mode) !== -1
            }

            function _isListed(mode) {
                return control.editMode
                    || showAdvanced
                    || mode === control._flightMode
                    || (!_isHidden(mode) && !_isAdvanced(mode))
            }

            function _needsConfirm(mode) {
                return _isReturnMode(mode) && activeVehicle.armed && activeVehicle.flying
            }

            function _pick(mode) {
                mainWindow.closeIndicatorDrawer()
                if (_needsConfirm(mode)) {
                    const guided = globals.guidedControllerFlyView
                    guided.confirmAction(guided.actionSetFlightMode, mode)
                } else {
                    control._requestMode(mode)
                }
            }

            function _setHidden(mode, hidden) {
                const remaining = hiddenFlightModesList.filter((item) => item !== mode)
                hiddenFlightModesList = hidden ? [ ...remaining, mode ] : remaining
                hiddenFlightModesFact.value = hiddenFlightModesList.join(",")
            }

            Component.onCompleted: {
                const hiddenFlightModesPropPrefix = activeVehicle.px4Firmware ? "px4HiddenFlightModes"
                                                  : activeVehicle.apmFirmware ? "apmHiddenFlightModes"
                                                  : ""
                const hiddenFlightModesProp = hiddenFlightModesPropPrefix + activeVehicle.vehicleClassInternalName()
                const editable = hiddenFlightModesPropPrefix !== "" && flightModeSettings.hasOwnProperty(hiddenFlightModesProp)
                control.allowEditMode = editable
                if (editable) {
                    hiddenFlightModesFact = flightModeSettings[hiddenFlightModesProp]
                    hiddenFlightModesList = hiddenFlightModesFact.value === "" ? [] : hiddenFlightModesFact.value.split(",")
                }
            }

            component ModeSection: Repeater {
                delegate: RowLayout {
                    Layout.fillWidth:   true
                    spacing:            ScreenTools.defaultFontPixelWidth
                    visible:            modeColumn._isListed(modelData)

                    OverlayMenuItem {
                        Layout.fillWidth:   true
                        text:               modelData
                        icon:               control._modeIcon(modelData)
                        description:        modeColumn._compact ? "" : control._modeDescription(modelData)
                        current:            modeColumn.activeVehicle && modeColumn.activeVehicle.flightMode === modelData
                        opacity:            modeColumn._isHidden(modelData) && !control.editMode ? 0.55 : 1

                        onClicked:      control.editMode ? modeColumn._setHidden(modelData, !modeColumn._isHidden(modelData)) : modeColumn._pick(modelData)
                        onPressAndHold: control.allowEditMode && modeColumn._setHidden(modelData, !modeColumn._isHidden(modelData))
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

            OverlayMenuSeparator { visible: moreModesItem.visible }

            OverlayMenuItem {
                id:         moreModesItem
                text:       qsTr("More modes")
                icon:       "/InstrumentValueIcons/dots-horizontal-triple.svg"
                visible:    modeColumn._moreAvailable
                onClicked:  modeColumn.showAdvanced = true
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
