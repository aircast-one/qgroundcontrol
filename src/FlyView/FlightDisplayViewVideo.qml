import QtQuick
import QtQuick.Controls
import QtQuick.Effects

import QGroundControl
import QGroundControl.FlyView
import QGroundControl.FlightMap
import QGroundControl.Controls

Item {
    id:     root
    clip:   true

    QGCPalette { id: qgcPal }

    property bool useSmallFont: true

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
    property bool   _showStreamLoader:  QGroundControl.videoManager.decoding
    property bool   _showUvcLoader:     QGroundControl.videoManager.isUvc

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

        // A placeholder has to be unmistakably a placeholder. This used to be a full-bleed
        // photograph of farmland - photorealistic aerial imagery in the exact place the
        // aircraft's camera feed goes, which a pilot glancing across the screen reads as a live
        // picture until they stop and read the pill. Absent content gets a neutral surface and
        // a symbol saying what is missing, never content that could be mistaken for the real
        // thing.
        Rectangle {
            id:             noVideo
            anchors.fill:   parent
            color:          "transparent"
            visible:        !(QGroundControl.videoManager.decoding)

            OverlayGlass {
                id:           noVideoGlass
                anchors.fill: parent
                radius:       0
            }

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

            readonly property bool receivingData: QGroundControl.videoManager.streaming

            readonly property string noVideoText: {
                const settings = QGroundControl.settingsManager.videoSettings
                const source   = settings.videoSource.rawValue
                if (source.indexOf("UDP") >= 0 || source.indexOf("MPEG-TS") >= 0) {
                    return qsTr("No video on UDP port %1").arg(settings.udpUrl.rawValue.split(":").pop())
                }
                const url = source.indexOf("RTSP") >= 0 ? settings.rtspUrl.rawValue
                          : source.indexOf("TCP")  >= 0 ? settings.tcpUrl.rawValue
                          : source.indexOf("WHEP") >= 0 ? settings.whepUrl.rawValue
                                                        : ""
                return url !== "" ? qsTr("No video from %1").arg(url) : qsTr("No video from %1").arg(source)
            }

            property int elapsedSecs: 0

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



        }

        Item {
            id:                 statusPill
            objectName:         "videoStatusPill"
            anchors.centerIn:   parent
            width:              pillContent.width
            height:             pillContent.height
            opacity:            noVideo.settled ? 1 : 0
            visible:            noVideo.visible && opacity > 0

            Behavior on opacity { NumberAnimation { duration: 250; easing.type: Easing.OutCubic } }

            Column {
                id:                 pillContent
                anchors.centerIn:   parent
                spacing:            ScreenTools.defaultFontPixelHeight / 2

                // Part of the stack, not a separate object hovering above a card: symbol,
                // title, cause, actions is one composition.
                QGCColoredImage {
                    anchors.horizontalCenter: parent.horizontalCenter
                    source:             "/InstrumentValueIcons/video-camera.svg"
                    color:              noVideoGlass.contentColor
                    opacity:            0.25
                    height:             ScreenTools.defaultFontPixelHeight * 4
                    width:              height
                    sourceSize.height:  height
                    fillMode:           Image.PreserveAspectFit
                    mipmap:             true
                }

                Row {
                    id:                         statusRow
                    anchors.horizontalCenter:   parent.horizontalCenter
                    spacing:                    ScreenTools.defaultFontPixelWidth

                    // Only while it is genuinely still trying. Once the title settles to
                    // "No Video Signal" a spinner beside it says the opposite of the words.
                    QGCSpinner {
                        anchors.verticalCenter: parent.verticalCenter
                        color:                  noVideoGlass.contentColor
                        visible:                noVideo.connecting && noVideo.streamEnabled &&
                                                    !noVideo.prolonged
                    }

                    QGCLabel {
                        id:                     noVideoLabel
                        anchors.verticalCenter: parent.verticalCenter
                        // Titles the state once it is clearly a state rather than a moment. The
                        // trailing ellipsis promises this is about to finish - true for the
                        // first few seconds, a lie by the time the detail line appears.
                        text:                   !noVideo.streamEnabled ? qsTr("Video off")
                                              : noVideo.prolonged      ? qsTr("No Video Signal")
                                                                       : noVideo.statusText
                        color:                  noVideoGlass.contentColor
                        font.pointSize:         ScreenTools.defaultFontPointSize
                    }
                }

                QGCLabel {
                    anchors.horizontalCenter:   parent.horizontalCenter
                    visible:                    noVideo.prolonged && noVideo.streamEnabled
                    color:                      Qt.alpha(noVideoGlass.contentColor, 0.6)
                    font.pointSize:             ScreenTools.smallFontPointSize
                    text: {
                        const what = noVideo.receivingData
                                         ? qsTr("Receiving data — waiting for video")
                                         : noVideo.noVideoText
                        return qsTr("%1 for %2").arg(what).arg(noVideo.elapsedText)
                    }
                }

                Row {
                    anchors.horizontalCenter:   parent.horizontalCenter
                    spacing:                    ScreenTools.defaultFontPixelWidth
                    visible:                    noVideo.prolonged && noVideo.streamEnabled

                    OverlayPill {
                        objectName: "videoRetryButton"
                        anchors.verticalCenter: parent.verticalCenter
                        accent:  qgcPal.colorBlue
                        text:    qsTr("Retry")
                        onClicked: {
                            noVideo.elapsedSecs = 0
                            noVideo.prolonged   = false
                            QGroundControl.videoManager.startVideo()
                        }
                    }

                    OverlayPill {
                        objectName: "videoSettingsButton"
                        anchors.verticalCenter: parent.verticalCenter
                        text:       qsTr("Video Settings")
                        onClicked:  mainWindow.showVideoSettings()
                    }
                }
            }
        }

    Rectangle {
        id:             videoBackground
        anchors.fill:   parent
        color:          "black"
        visible:        _showStreamLoader || _showUvcLoader
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
        Loader {
            id:                 videoStreamLoader
            anchors.fill:       videoContentArea
            visible:            _showStreamLoader
            sourceComponent:    videoOutputComponent

            property bool videoDisabled: QGroundControl.settingsManager.videoSettings.videoSource.rawValue === QGroundControl.settingsManager.videoSettings.disabledVideoSource
        }
        Component {
            id: videoOutputComponent
            FlightDisplayViewVideoOutput {
            }
        }
        //-- UVC Video (USB Camera or Video Device)
        Loader {
            id:             cameraLoader
            anchors.fill:   videoContentArea
            visible:        _showUvcLoader
            source:         _showUvcLoader ? "qrc:/qml/QGroundControl/FlyView/FlightDisplayViewUVC.qml" : "qrc:/qml/QGroundControl/FlyView/FlightDisplayViewDummy.qml"
        }

        Item {
            id:                 videoContentArea
            height:             parent.getHeight()
            width:              parent.getWidth()
            anchors.centerIn:   parent
            visible:           _showStreamLoader || _showUvcLoader

            // grid lines
            Item {
                anchors.fill:   parent
                visible:        _showGrid && !QGroundControl.videoManager.fullScreen

                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    height: parent.height
                    width:  1
                    x:      parent.width * 0.33
                }
                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    height: parent.height
                    width:  1
                    x:      parent.width * 0.66
                }
                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    width:  parent.width
                    height: 1
                    y:      parent.height * 0.33
                }
                Rectangle {
                    color:  Qt.rgba(1,1,1,0.5)
                    width:  parent.width
                    height: 1
                    y:      parent.height * 0.66
                }
            }
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
            height:             _camera ? (_camera.thermalMode === MavlinkCameraControlInterface.THERMAL_FULL ? parent.height : (_camera.thermalMode === MavlinkCameraControlInterface.THERMAL_PIP ? ScreenTools.defaultFontPixelHeight * 12 : parent.height * _thermalHeightFactor)) : 0
            anchors.centerIn:   parent
            visible:            QGroundControl.videoManager.hasThermal && _camera && _camera.thermalMode !== MavlinkCameraControlInterface.THERMAL_OFF
            function pipOrNot() {
                if(_camera) {
                    if(_camera.thermalMode === MavlinkCameraControlInterface.THERMAL_PIP) {
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
                function onThermalModeChanged() { thermalItem.pipOrNot() }
            }
            onVisibleChanged: {
                thermalItem.pipOrNot()
            }
            Loader {
                id:             thermalVideo
                anchors.fill:   parent
                opacity:        _camera ? (_camera.thermalMode === MavlinkCameraControlInterface.THERMAL_BLEND ? _camera.thermalOpacity / 100 : 1.0) : 0
                sourceComponent: thermalOutputComponent
                onLoaded: { if (item) item.objectName = "thermalVideo" }

                Component {
                    id: thermalOutputComponent
                    FlightDisplayViewVideoOutput {}
                }
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
