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
    height: visible ? Math.max(_slimHeight, _dockedBarHeight) + _topInset : 0
    color:  "transparent"

    readonly property real _slimHeight: ScreenTools.toolbarHeight
    readonly property real _topInset:   ScreenTools.defaultFontPixelHeight * 0.75

    property Item   dockedTelemetryBar

    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property bool   _communicationLost: _activeVehicle ? _activeVehicle.vehicleLinkManager.communicationLost : false
    property color  _mainStatusBGColor: "transparent"
    property real   _toolsSpacing:      ScreenTools.defaultFontPixelWidth

    readonly property real _clusterHeight: Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.5)

    readonly property real dockAreaLeft: toolIndicators.x

    readonly property real _dockedBarHeight: dockedTelemetryBar ? dockedTelemetryBar.height : 0
    readonly property real _dockedBarLeft:   dockedTelemetryBar ? _root.mapFromItem(dockedTelemetryBar.parent, dockedTelemetryBar.x, 0).x : width
    readonly property real _dockedBarRight:  dockedTelemetryBar ? _dockedBarLeft + dockedTelemetryBar.width : 0

    function dropMainStatusIndicatorTool() {
        mainStatusIndicator.dropMainStatusIndicator();
    }

    QGCPalette { id: qgcPal }

    Rectangle {
        id:             bottomDivider
        anchors.left:   parent.left
        anchors.right:  parent.right
        anchors.bottom: parent.bottom
        height:         1
        color:          "black"
        visible:        qgcPal.globalTheme === QGCPalette.Light
    }



    Component.onCompleted: {
        mainWindow.registerWindowDragExclusion(viewButtonRow)
        mainWindow.registerWindowDragExclusion(toolIndicators)
        mainWindow.registerWindowDragExclusion(overflowButton)
    }

    RowLayout {
        id:                     viewButtonRow
        anchors.leftMargin:     mainWindow.windowChromeLeftInset + ScreenTools.defaultFontPixelWidth * 3
        anchors.left:           parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: _topInset / 2
        spacing:                ScreenTools.defaultFontPixelWidth * 2

        OverlayCapsule {
            id:                     viewSwitch
            Layout.preferredHeight: _clusterHeight
            Layout.preferredWidth:  viewSwitchControl.implicitWidth
            radius:                 height / 2

            OverlayViewSwitch {
                id:           viewSwitchControl
                objectName:   "viewSwitch"
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

        MainStatusIndicator {
            id:                     mainStatusIndicator
            Layout.preferredHeight: _clusterHeight
        }

        OverlayActivityCapsule {
            id:                     activityChip
            objectName:             "vehicleActivityChip"
            Layout.preferredHeight: _clusterHeight
            Layout.preferredWidth:  width
            progress:               _loading && _activeVehicle.loadProgress > 0 ? _activeVehicle.loadProgress : -1
            done:                   !_loading
            text:                   _loading ? qsTr("Loading vehicle") : qsTr("Connected")
            opacity:                _activeVehicle && (_loading || lingerTimer.running) ? 1 : 0
            visible:                opacity > 0

            readonly property bool _loading: _activeVehicle ? !_activeVehicle.initialConnectComplete : false

            on_LoadingChanged: if (!_loading) lingerTimer.restart()

            Behavior on opacity { NumberAnimation { duration: 200 } }

            Timer {
                id:       lingerTimer
                interval: 1800
            }
        }

        Loader {
            id:                 flightModeIndicatorLoader
            objectName:         "flightModeIndicator"
            Layout.alignment:   Qt.AlignVCenter
            source:             _flightModeIndicatorUrl
            visible:            status === Loader.Ready && item.showIndicator

            onLoaded: item.fontPointSize = ScreenTools.defaultFontPointSize

            readonly property url _flightModeIndicatorUrl: {
                if (!_activeVehicle) {
                    return ""
                }
                const urls = _activeVehicle.toolIndicators
                for (var i = 0; i < urls.length; i++) {
                    if (urls[i].toString().indexOf("FlightModeIndicator") >= 0) {
                        return urls[i]
                    }
                }
                return ""
            }
        }

        QGCButton {
            id:                 disconnectButton
            text:               qsTr("Disconnect")
            onClicked:          _activeVehicle.closeVehicle()
            visible:            _activeVehicle && _communicationLost
        }
    }

    OverlayRoundButton {
        id:                             overflowButton
        objectName:                     "toolSelectButton"
        anchors.right:                  parent.right
        anchors.rightMargin:            mainWindow.windowChromeRightInset + ScreenTools.defaultFontPixelWidth * 3
        anchors.verticalCenter:         parent.verticalCenter
        anchors.verticalCenterOffset:   _topInset / 2
        icon:                           "/InstrumentValueIcons/dots-horizontal-triple.svg"
        onClicked:                      mainWindow.showToolSelectDialog(overflowButton)
    }

    FlyViewToolBarIndicators {
        id:                             toolIndicators
        anchors.right:                  overflowButton.left
        anchors.rightMargin:            ScreenTools.defaultFontPixelWidth * 2
        anchors.verticalCenter:         parent.verticalCenter
        anchors.verticalCenterOffset:   _topInset / 2
        height:                         ScreenTools.defaultFontPixelHeight * 1.8
    }
}
