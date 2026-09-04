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
    // No band. The status sits directly on the picture, held legible over bright imagery by a
    // soft scrim rather than by a slab of chrome.
    color:  "transparent"

    // Full height: the strip carries icon-and-value pairs sized to be read in flight, and
    // squeezing it left them cramped against the picture.
    readonly property real _slimHeight: ScreenTools.toolbarHeight
    readonly property real _topInset:   ScreenTools.defaultFontPixelHeight * 0.75

    property Item   dockedTelemetryBar

    property var    _activeVehicle:     QGroundControl.multiVehicleManager.activeVehicle
    property bool   _communicationLost: _activeVehicle ? _activeVehicle.vehicleLinkManager.communicationLost : false
    property color  _mainStatusBGColor: "transparent"
    property real   _toolsSpacing:      ScreenTools.defaultFontPixelWidth

    readonly property real dockAreaLeft: toolIndicators.x

    readonly property real _dockedBarHeight: dockedTelemetryBar ? dockedTelemetryBar.height : 0
    readonly property real _dockedBarLeft:   dockedTelemetryBar ? _root.mapFromItem(dockedTelemetryBar.parent, dockedTelemetryBar.x, 0).x : width
    readonly property real _dockedBarRight:  dockedTelemetryBar ? _dockedBarLeft + dockedTelemetryBar.width : 0

    function dropMainStatusIndicatorTool() {
        mainStatusIndicator.dropMainStatusIndicator();
    }

    QGCPalette { id: qgcPal }

    // Tall enough to hold the text against bright imagery; a scrim only as tall as the text
    // leaves it fighting the picture directly underneath.
    Rectangle {
        anchors.left:   parent.left
        anchors.right:  parent.right
        anchors.top:    parent.top
        height:         parent.height * 1.5
        gradient: Gradient {
            GradientStop { position: 0;    color: Qt.rgba(0, 0, 0, 0.85) }
            GradientStop { position: 0.6;  color: Qt.rgba(0, 0, 0, 0.6) }
            GradientStop { position: 1;    color: "transparent" }
        }
    }

    /// Bottom single pixel divider
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
        mainWindow.registerWindowDragExclusion(largeProgressBar)
    }

    RowLayout {
        id:                     viewButtonRow
        anchors.leftMargin:     mainWindow.windowChromeLeftInset + ScreenTools.defaultFontPixelWidth * 3
        anchors.left:           parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.verticalCenterOffset: _topInset / 2
        spacing:                ScreenTools.defaultFontPixelWidth * 2

        QGCToolBarButton {
            id:                     currentButton
            Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 1.8
            icon.source:            "/res/QGCLogoFull.svg"
            logo:                   true
            onClicked:              mainWindow.showToolSelectDialog()
        }

        MainStatusIndicator {
            id: mainStatusIndicator
            Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 1.8
        }

        // Flight mode is flight-critical state: it sits with "Ready To Fly" in the fixed
        // cluster, sized for the slim bar, instead of overflowing the scrolling strip.
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

    // QGC's own indicators: each one opens a detail drawer on click (cell voltages, GPS
    // quality, RSSI). A display-only strip looked tidier and quietly removed all of that.
    FlyViewToolBarIndicators {
        id:                             toolIndicators
        anchors.right:                  parent.right
        anchors.rightMargin:            mainWindow.windowChromeRightInset + ScreenTools.defaultFontPixelWidth * 3
        anchors.verticalCenter:         parent.verticalCenter
        anchors.verticalCenterOffset:   _topInset / 2
        height:                         ScreenTools.defaultFontPixelHeight * 1.8
    }

    // Small parameter download progress bar
    Rectangle {
        anchors.bottom: parent.bottom
        height:         _root.height * 0.05
        width:          _activeVehicle ? _activeVehicle.loadProgress * parent.width : 0
        color:          qgcPal.colorGreen
        visible:        !largeProgressBar.visible
    }

    // Large parameter download progress bar
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
