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
        mainWindow.registerWindowDragExclusion(largeProgressBar)
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
            objectName:             "viewSwitch"
            Layout.preferredHeight: _clusterHeight
            Layout.preferredWidth:  viewSwitchControl.implicitWidth
            radius:                 height / 2

            OverlayViewSwitch {
                id:           viewSwitchControl
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

    Rectangle {
        anchors.bottom: parent.bottom
        height:         _root.height * 0.05
        width:          _activeVehicle ? _activeVehicle.loadProgress * parent.width : 0
        color:          qgcPal.colorGreen
        visible:        !largeProgressBar.visible
    }

    Rectangle {
        id:             largeProgressBar
        anchors.bottom: parent.bottom
        anchors.left:   parent.left
        anchors.right:  parent.right
        height:         parent.height
        color:          qgcPal.window
        visible:        _showLargeProgress

        property bool _initialDownloadComplete: _activeVehicle ? _activeVehicle.initialConnectComplete : true
        property bool _userHide:                false
        property bool _showLargeProgress:       !_initialDownloadComplete && !_userHide && qgcPal.globalTheme === QGCPalette.Light

        Connections {
            target:                 QGroundControl.multiVehicleManager
            function onActiveVehicleChanged(activeVehicle) { largeProgressBar._userHide = false }
        }

        Rectangle {
            anchors.top:    parent.top
            anchors.bottom: parent.bottom
            width:          _activeVehicle ? _activeVehicle.loadProgress * parent.width : 0
            color:          qgcPal.colorGreen
        }

        QGCLabel {
            anchors.centerIn:   parent
            text:               qsTr("Downloading")
            font.pointSize:     ScreenTools.largeFontPointSize
        }

        QGCLabel {
            anchors.margins:    _margin
            anchors.right:      parent.right
            anchors.bottom:     parent.bottom
            text:               qsTr("Click anywhere to hide")

            property real _margin: ScreenTools.defaultFontPixelWidth / 2
        }

        MouseArea {
            anchors.fill:   parent
            onClicked:      largeProgressBar._userHide = true
        }
    }
}
