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

import QGroundControl
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.Vehicle
import QGroundControl.Controllers

Item {
    id:     root
    clip:   true

    property bool useSmallFont: true

    // The collision rig needs a handle on the status pill to keep telemetry chips off it.
    readonly property alias statusPill: statusPill

    property double _ar:                (QGroundControl.videoManager.gstreamerEnabled && QGroundControl.videoManager.videoSize.width > 0 && QGroundControl.videoManager.videoSize.height > 0)
                                            ? QGroundControl.videoManager.videoSize.width / QGroundControl.videoManager.videoSize.height
                                            : QGroundControl.videoManager.aspectRatio
    property bool   _showGrid:          QGroundControl.settingsManager.videoSettings.gridLines.rawValue
    property var    _dynamicCameras:    globals.activeVehicle ? globals.activeVehicle.cameraManager : null
    property bool   _connected:         globals.activeVehicle ? !globals.activeVehicle.communicationLost : false
    property int    _curCameraIndex:    _dynamicCameras ? _dynamicCameras.currentCamera : 0
    property bool   _isCamera:          _dynamicCameras ? _dynamicCameras.cameras.count > 0 : false
    property var    _camera:            _isCamera ? _dynamicCameras.cameras.get(_curCameraIndex) : null
    property bool   _hasZoom:           _camera && _camera.hasZoom
    property int    _fitMode:           QGroundControl.settingsManager.videoSettings.videoFit.rawValue

    property bool   _isMode_FIT_WIDTH:  _fitMode === 0
    property bool   _isMode_FIT_HEIGHT: _fitMode === 1
    property bool   _isMode_FILL:       _fitMode === 2
    property bool   _isMode_NO_CROP:    _fitMode === 3

    function getWidth() {
        return videoBackground.getWidth()
    }
    function getHeight() {
        return videoBackground.getHeight()
    }

    property double _thermalHeightFactor: 0.85

        Image {
            id:             noVideo
            anchors.fill:   parent
            source:         "/res/NoVideoBackground.jpg"
            fillMode:       Image.PreserveAspectCrop
            visible:        !(QGroundControl.videoManager.decoding)

            property string statusText: {
                var statuses = QGroundControl.videoManager.cameraStatuses
                var active = QGroundControl.videoManager.activeVideoSource
                return active >= 0 && active < statuses.length ? statuses[active] : ""
            }

            readonly property bool connecting: {
                var flags  = QGroundControl.videoManager.cameraConnecting
                var active = QGroundControl.videoManager.activeVideoSource
                return active >= 0 && active < flags.length ? flags[active] : false
            }

            readonly property bool streamEnabled: QGroundControl.settingsManager.videoSettings.streamEnabled.rawValue

            // "Nothing is arriving" and "data is arriving but will not decode" look identical
            // on screen but have opposite fixes, so the pill has to tell them apart.
            readonly property bool receivingData: QGroundControl.videoManager.streaming

            readonly property string sourceLabel: {
                const settings = QGroundControl.settingsManager.videoSettings
                const source   = settings.videoSource.rawValue
                const url      = source.indexOf("RTSP") >= 0 ? settings.rtspUrl.rawValue
                               : source.indexOf("TCP")  >= 0 ? settings.tcpUrl.rawValue
                               : source.indexOf("UDP")  >= 0 ? settings.udpUrl.rawValue
                               : source.indexOf("WHEP") >= 0 ? settings.whepUrl.rawValue
                                                             : ""
                return url !== "" ? url : source
            }

            property int elapsedSecs: 0

            // "3565 s" is a number the reader has to divide. Anything past a minute reads in
            // the units people wait in.
            readonly property string elapsedText: _elapsedText(elapsedSecs)

            function _elapsedText(secs) {
                if (secs < 60) {
                    return qsTr("%1 s").arg(secs)
                }
                if (secs < 3600) {
                    return qsTr("%1 min").arg(Math.floor(secs / 60))
                }
                return qsTr("%1 h %2 min").arg(Math.floor(secs / 3600)).arg(Math.floor((secs % 3600) / 60))
            }

            Timer {
                interval:       1000
                repeat:         true
                running:        noVideo.visible && noVideo.streamEnabled
                onTriggered:    noVideo.elapsedSecs++
            }

            property bool settled: false

            property bool prolonged: false

            onVisibleChanged: if (!visible) { settled = false; prolonged = false; elapsedSecs = 0 }

            Timer {
                interval:       400
                running:        noVideo.visible && !noVideo.settled
                onTriggered:    noVideo.settled = true
            }

            Timer {
                interval:       8000
                running:        noVideo.visible && !noVideo.prolonged
                onTriggered:    noVideo.prolonged = true
            }

            QGCPalette { id: qgcPal }

            Rectangle {
                anchors.fill:   parent
                color:          "black"
                opacity:        0.45
            }

            OverlayCapsule {
                id:                 statusPill
                objectName:         "videoStatusPill"
                frosted:            false
                anchors.centerIn:   parent
                width:              pillContent.width + ScreenTools.defaultFontPixelHeight * 1.6
                height:             pillContent.height + ScreenTools.defaultFontPixelHeight * 0.9

                Behavior on width  { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                Behavior on height { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }
                opacity:            noVideo.settled ? 1 : 0
                // The rig reads visibility, not opacity. Left as a permanently visible item
                // it would shove the chips aside during the second before the pill fades in.
                visible:            opacity > 0

                Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }

                Column {
                    id:                 pillContent
                    anchors.centerIn:   parent
                    spacing:            ScreenTools.defaultFontPixelHeight / 5

                Row {
                    id:                 statusRow
                    anchors.horizontalCenter: parent.horizontalCenter
                    spacing:            ScreenTools.defaultFontPixelWidth

                    QGCSpinner {
                        anchors.verticalCenter: parent.verticalCenter
                        color:                  qgcPal.text
                        visible:                noVideo.connecting && noVideo.streamEnabled
                    }

                    QGCLabel {
                        id:                     noVideoLabel
                        anchors.verticalCenter: parent.verticalCenter
                        text:                   noVideo.streamEnabled ? noVideo.statusText : qsTr("Video off")
                        color:                  qgcPal.text
                        font.pointSize:         useSmallFont ? ScreenTools.defaultFontPointSize : ScreenTools.mediumFontPointSize
                    }
                }

                    QGCLabel {
                        anchors.horizontalCenter:   parent.horizontalCenter
                        visible:                    noVideo.prolonged && noVideo.streamEnabled
                        color:                      qgcPal.colorGrey
                        font.pointSize:             ScreenTools.smallFontPointSize
                        text: {
                            const what = noVideo.receivingData
                                             ? qsTr("Receiving data — waiting for video")
                                             : qsTr("Nothing arriving from %1").arg(noVideo.sourceLabel)
                            return qsTr("%1  ·  %2").arg(what).arg(noVideo.elapsedText)
                        }
                    }

                    Row {
                        anchors.horizontalCenter:   parent.horizontalCenter
                        spacing:                    ScreenTools.defaultFontPixelWidth
                        visible:                    noVideo.prolonged && noVideo.streamEnabled

                        // Retry resolves this for most people, so it carries the weight; two
                        // identical-looking buttons made the reader pick one at random.
                        OverlayPill {
                            anchors.verticalCenter: parent.verticalCenter
                            frosted: false
                            text: qsTr("Retry")
                            onClicked: {
                                noVideo.elapsedSecs = 0
                                noVideo.prolonged   = false
                                QGroundControl.videoManager.startVideo()
                            }
                        }

                        OverlayMenuItem {
                            anchors.verticalCenter: parent.verticalCenter
                            text:       qsTr("Video Settings")
                            onClicked:  mainWindow.showSettingsTool(qsTr("Video"))
                        }
                    }
                }
            }
        }

    Rectangle {
        id:             videoBackground
        anchors.fill:   parent
        color:          "black"
        visible:        QGroundControl.videoManager.decoding
        function getWidth() {
            if(_ar != 0.0){
                if(_isMode_FIT_HEIGHT 
                        || (_isMode_FILL && (root.width/root.height < _ar))
                        || (_isMode_NO_CROP && (root.width/root.height > _ar))){
                    return root.height * _ar
                }
            }
            return root.width
        }
        function getHeight() {
            if(_ar != 0.0){
                if(_isMode_FIT_WIDTH 
                        || (_isMode_FILL && (root.width/root.height > _ar)) 
                        || (_isMode_NO_CROP && (root.width/root.height < _ar))){
                    return root.width * (1 / _ar)
                }
            }
            return root.height
        }
        Component {
            id: videoBackgroundComponent
            QGCVideoBackground {
                id:             videoContent
                objectName:     "videoContent"

                Connections {
                    target: QGroundControl.videoManager
                    function onImageFileChanged(filename) {
                        videoContent.grabToImage(function(result) {
                            if (!result.saveToFile(filename)) {
                                console.error('Error capturing video frame');
                            }
                        });
                    }
                }

                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    height: parent.height
                    width:  1
                    x:      parent.width * 0.33
                    visible: _showGrid && !QGroundControl.videoManager.fullScreen
                }
                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    height: parent.height
                    width:  1
                    x:      parent.width * 0.66
                    visible: _showGrid && !QGroundControl.videoManager.fullScreen
                }
                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    width:  parent.width
                    height: 1
                    y:      parent.height * 0.33
                    visible: _showGrid && !QGroundControl.videoManager.fullScreen
                }
                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    width:  parent.width
                    height: 1
                    y:      parent.height * 0.66
                    visible: _showGrid && !QGroundControl.videoManager.fullScreen
                }
            }
        }
        Loader {
            height:             parent.getHeight()
            width:              parent.getWidth()
            anchors.centerIn:   parent
            visible:            QGroundControl.videoManager.decoding
            sourceComponent:    videoBackgroundComponent

            property bool videoDisabled: QGroundControl.settingsManager.videoSettings.videoSource.rawValue === QGroundControl.settingsManager.videoSettings.disabledVideoSource
        }

        DetectionOverlayVideo {
            width:            videoBackground.getWidth()
            height:           videoBackground.getHeight()
            anchors.centerIn: parent
            visible:          QGroundControl.videoManager.decoding
        }

        Item {
            id:                 thermalItem
            width:              height * QGroundControl.videoManager.thermalAspectRatio
            height:             _camera ? (_camera.thermalMode === MavlinkCameraControl.THERMAL_FULL ? parent.height : (_camera.thermalMode === MavlinkCameraControl.THERMAL_PIP ? ScreenTools.defaultFontPixelHeight * 12 : parent.height * _thermalHeightFactor)) : 0
            anchors.centerIn:   parent
            visible:            QGroundControl.videoManager.hasThermal && _camera.thermalMode !== MavlinkCameraControl.THERMAL_OFF
            function pipOrNot() {
                if(_camera) {
                    if(_camera.thermalMode === MavlinkCameraControl.THERMAL_PIP) {
                        anchors.centerIn    = undefined
                        anchors.top         = parent.top
                        anchors.topMargin   = mainWindow.header.height + (ScreenTools.defaultFontPixelHeight * 0.5)
                        anchors.left        = parent.left
                        anchors.leftMargin  = ScreenTools.defaultFontPixelWidth * 12
                    } else {
                        anchors.top         = undefined
                        anchors.topMargin   = undefined
                        anchors.left        = undefined
                        anchors.leftMargin  = undefined
                        anchors.centerIn    = parent
                    }
                }
            }
            Connections {
                target:                 _camera
                onThermalModeChanged:   thermalItem.pipOrNot()
            }
            onVisibleChanged: {
                thermalItem.pipOrNot()
            }
            QGCVideoBackground {
                id:             thermalVideo
                objectName:     "thermalVideo"
                anchors.fill:   parent
                opacity:        _camera ? (_camera.thermalMode === MavlinkCameraControl.THERMAL_BLEND ? _camera.thermalOpacity / 100 : 1.0) : 0
            }
        }
        PinchArea {
            id:             pinchZoom
            enabled:        _hasZoom
            anchors.fill:   parent
            onPinchStarted: pinchZoom.zoom = 0
            onPinchUpdated: {
                if(_hasZoom) {
                    var z = 0
                    if(pinch.scale < 1) {
                        z = Math.round(pinch.scale * -10)
                    } else {
                        z = Math.round(pinch.scale)
                    }
                    if(pinchZoom.zoom != z) {
                        _camera.stepZoom(z)
                    }
                }
            }
            property int zoom: 0
        }

    }
}
