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
import QtQuick.Dialogs

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.MultiVehicleManager
import QGroundControl.ScreenTools
import QGroundControl.Controllers
import QGroundControl.FlightDisplay

Rectangle {
    id:         _root
    objectName: "flyViewToolBar"
    width:      parent.width
    height: visible ? _slimHeight + _topInset : 0
    color:  "transparent"

    required property var overlayRig

    readonly property real _slimHeight: ScreenTools.toolbarHeight
    readonly property real _topInset:   ScreenTools.defaultFontPixelHeight * 0.75 + ScreenTools.safeAreaTop

    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property bool   _communicationLost: _activeVehicle ? _activeVehicle.vehicleLinkManager.communicationLost : false
    property real   _toolsSpacing:      ScreenTools.defaultFontPixelWidth

    readonly property real _clusterHeight: Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.5)

    readonly property real _gutter:           ScreenTools.defaultFontPixelWidth * 2
    readonly property real _indicatorPadding: ScreenTools.defaultFontPixelWidth

    readonly property real _room:           Math.max(0, toolButtonRow.x - viewButtonRow.x - _gutter)
    readonly property real _statusFloor:    mainStatusIndicator.legibleWidth
    readonly property real _leftFloor:      viewSwitchControl.implicitWidth + viewButtonRow.spacing + _statusFloor
    readonly property real _indicatorFloor: ScreenTools.defaultFontPixelHeight * 3
    readonly property real _indicatorFit:   Math.min(toolIndicators.implicitWidth,
                                                     _room - _leftFloor - _gutter - _indicatorPadding * 2)
    readonly property real _indicatorRoom:  _indicatorFit >= _indicatorFloor ? _indicatorFit : 0
    readonly property real _indicatorCost:  toolIndicators.width > 0 ? toolIndicators.width + _indicatorPadding * 2 + _gutter : 0
    readonly property real _viewButtonRoom: Math.max(0, _room - _indicatorCost)

    function _fitsBesideStatus(extra) {
        return _viewButtonRoom >= viewSwitchControl.implicitWidth + viewButtonRow.spacing * 2 + _statusFloor + extra
    }

    function dropMainStatusIndicatorTool() {
        mainStatusIndicator.dropMainStatusIndicator();
    }

    QGCPalette { id: qgcPal }

    Component.onCompleted: {
        mainWindow.registerWindowDragExclusion(viewButtonRow)
        mainWindow.registerWindowDragExclusion(toolIndicators)
        mainWindow.registerWindowDragExclusion(toolButtonRow)
    }

    RowLayout {
        id:                     viewButtonRow
        anchors.leftMargin:     mainWindow.windowChromeLeftInset + ScreenTools.safeAreaLeft + ScreenTools.defaultFontPixelWidth * 3
        anchors.left:           parent.left
        width:                  Math.min(implicitWidth, _root._viewButtonRoom)
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: _topInset / 2
        spacing:                ScreenTools.defaultFontPixelWidth * 2

        OverlayEditSlot {
            rig:                    overlayRig
            swallowsTaps:           true
            editKey:                "viewSwitch"
            Layout.preferredHeight: _clusterHeight
            Layout.preferredWidth:  viewSwitchControl.implicitWidth
            Layout.minimumWidth:    Layout.preferredWidth

            OverlayCapsule {
                id:           viewSwitch
                anchors.fill: parent
                radius:       height / 2

                OverlayViewSwitch {
                    id:           viewSwitchControl
                    objectName:   "viewSwitch"
                    contentColor: viewSwitch.contentColor
                    anchors.fill: parent
                    options:      [qsTr("Fly"), qsTr("Plan")]
                    currentIndex: mainWindow.flyViewActive ? 0 : 1
                    onActivated:  (index) => {
                        if (mainWindow.allowViewSwitch()) {
                            index === 0 ? mainWindow.showFlyView() : mainWindow.showPlanView()
                        }
                    }
                }
            }
        }

        OverlayEditSlot {
            rig:                    overlayRig
            swallowsTaps:           true
            editKey:                "mainStatusIndicator"
            Layout.preferredHeight: _clusterHeight
            Layout.fillWidth:       true
            Layout.preferredWidth:  mainStatusIndicator.implicitWidth
            Layout.maximumWidth:    mainStatusIndicator.implicitWidth
            Layout.minimumWidth:    mainStatusIndicator.minimumWidth

            MainStatusIndicator {
                id:           mainStatusIndicator
                anchors.fill: parent
            }
        }

        OverlayEditSlot {
            rig:                    overlayRig
            swallowsTaps:           true
            editKey:                "vehicleActivityChip"
            available:              activityChip.opacity > 0 && _root._fitsBesideStatus(activityChip.width)
            Layout.preferredHeight: _clusterHeight
            Layout.preferredWidth:  activityChip.width
            Layout.minimumWidth:    Layout.preferredWidth

            OverlayActivityCapsule {
                id:          activityChip
                objectName:  "vehicleActivityChip"
                height:      parent.height
                progress:    _loading && _activeVehicle.loadProgress > 0 ? _activeVehicle.loadProgress : -1
                done:        !_loading
                text:        _loading ? qsTr("Loading vehicle") : qsTr("Connected")
                opacity:     _activeVehicle && (_loading || lingerTimer.running) ? 1 : 0

                readonly property bool _loading: _activeVehicle ? !_activeVehicle.initialConnectComplete : false

                on_LoadingChanged: if (!_loading) lingerTimer.restart()

                Behavior on opacity { NumberAnimation { duration: 200 } }

                Timer {
                    id:       lingerTimer
                    interval: 1800
                }
            }
        }

        OverlayEditSlot {
            rig:                    overlayRig
            swallowsTaps:           true
            editKey:                "FlightModeIndicator"
            available:              flightModeIndicatorLoader.status === Loader.Ready && flightModeIndicatorLoader.item.showIndicator &&
                                        _root._fitsBesideStatus(flightModeIndicatorLoader.width)
            Layout.alignment:       Qt.AlignVCenter
            Layout.preferredWidth:  flightModeIndicatorLoader.width
            Layout.preferredHeight: flightModeIndicatorLoader.height
            Layout.minimumWidth:    Layout.preferredWidth

            Loader {
                id:         flightModeIndicatorLoader
                objectName: "flightModeIndicator"
                source:     _flightModeIndicatorUrl

                onLoaded: item.fontPointSize = ScreenTools.defaultFontPointSize

                readonly property url _flightModeIndicatorUrl: {
                    if (!_activeVehicle) {
                        return ""
                    }
                    const match = _activeVehicle.toolIndicators
                                      .find((url) => url.toString().indexOf("FlightModeIndicator") >= 0)
                    return match ? match : ""
                }
            }
        }

        QGCButton {
            id:                 disconnectButton
            text:               qsTr("Disconnect")
            onClicked:          _activeVehicle.closeVehicle()
            visible:            _activeVehicle && _communicationLost
        }
    }

    Row {
        id:                             toolButtonRow
        anchors.right:                  parent.right
        anchors.rightMargin:            mainWindow.windowChromeRightInset + ScreenTools.safeAreaRight + ScreenTools.defaultFontPixelWidth * 3
        anchors.verticalCenter:         parent.verticalCenter
        anchors.verticalCenterOffset:   _topInset / 2
        spacing:                        ScreenTools.defaultFontPixelWidth

        OverlayEditSlot {
            rig:     overlayRig
            swallowsTaps:           true
            editKey: "analyzeButton"
            width:   analyzeButton.width
            height:  analyzeButton.height

            OverlayRoundButton {
                id:         analyzeButton
                objectName: "analyzeButton"
                icon:       "/InstrumentValueIcons/chart-bar.svg"
                onClicked:  if (mainWindow.allowViewSwitch()) mainWindow.showAnalyzeTool()
            }
        }

        OverlayEditSlot {
            rig:        overlayRig
            swallowsTaps:           true
            editKey:    "settingsButton"
            available:  !QGroundControl.corePlugin.options.combineSettingsAndSetup
            width:      settingsButton.width
            height:     settingsButton.height

            OverlayRoundButton {
                id:         settingsButton
                objectName: "settingsButton"
                icon:       "/InstrumentValueIcons/cog.svg"
                onClicked:  if (mainWindow.allowViewSwitch()) mainWindow.showSettingsTool()
            }
        }
    }

    OverlayCapsule {
        anchors.fill:    toolIndicators
        anchors.margins: -_root._indicatorPadding
        radius:          height / 2
        z:               toolIndicators.z - 1
        visible:         toolIndicators.width > 0
    }

    FlyViewToolBarIndicators {
        id:                             toolIndicators
        objectName:                     "flyViewToolIndicators"
        anchors.right:                  toolButtonRow.left
        anchors.rightMargin:            _root._gutter + _root._indicatorPadding
        anchors.verticalCenter:         parent.verticalCenter
        anchors.verticalCenterOffset:   _topInset / 2
        height:                         ScreenTools.defaultFontPixelHeight * 1.8
        width:                          Math.max(0, _root._indicatorRoom)
        clip:                           true
    }
}
