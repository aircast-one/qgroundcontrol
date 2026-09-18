import QtQuick
import QtMultimedia

import QGroundControl
import QGroundControl.Controls

Item {
    id: _root

    property Item pipView
    property Item pipState: videoPipState

    PipState {
        id:         videoPipState
        pipView:    _root.pipView
        isDark:     true

        onWindowAboutToOpen: {
            QGroundControl.videoManager.stopVideo()
            videoStartDelay.start()
        }

        onWindowAboutToClose: {
            QGroundControl.videoManager.stopVideo()
            videoStartDelay.start()
        }

        onStateChanged: {
            if (pipState.state !== pipState.fullState) {
                QGroundControl.videoManager.fullScreen = false
            }
        }
    }

    Timer {
        id:           videoStartDelay
        interval:     2000;
        running:      false
        repeat:       false
        onTriggered:  QGroundControl.videoManager.startVideo()
    }

    //-- Video Streaming
    FlightDisplayViewVideo {
        id:             videoStreaming
        anchors.fill:   parent
        useSmallFont:   _root.pipState.state !== _root.pipState.fullState
        visible:        QGroundControl.videoManager.isStreamSource || QGroundControl.videoManager.isUvc
    }

    QGCLabel {
        text: qsTr("Double-click to exit full screen")
        font.pointSize: ScreenTools.largeFontPointSize
        visible: QGroundControl.videoManager.fullScreen
        anchors.centerIn: parent

        onVisibleChanged: {
            if (visible) {
                labelAnimation.start()
            }
        }

        PropertyAnimation on opacity {
            id: labelAnimation
            duration: 10000
            from: 1.0
            to: 0.0
            easing.type: Easing.InExpo
        }
    }

    OnScreenGimbalController {
        id:                      onScreenGimbalController
        anchors.fill:            parent
        cameraTrackingEnabled:   !!(videoStreaming._camera && videoStreaming._camera.trackingEnabled)
    }

    OnScreenCameraTrackingController {
        id:                      cameraTrackingController
        anchors.fill:            parent
        camera:                  videoStreaming._camera
        videoWidth:              videoStreaming.getWidth()
        videoHeight:             videoStreaming.getHeight()
    }

    MouseArea {
        id:                         flyViewVideoMouseArea
        anchors.fill:               parent
        enabled:                    pipState.state === pipState.fullState

        property real _pressX:      0
        property real _pressY:      0
        property bool _dragging:    false
        property bool _doubleClicked: false
        readonly property real _dragThreshold: 10

        // Defer single-click handling so a double-click (fullscreen toggle) doesn't also
        // fire an unintended gimbal click-to-point/tracking command on its first click.
        Timer {
            id:         singleClickTimer
            interval:   Qt.styleHints.mouseDoubleClickInterval
            repeat:     false

            property real clickX: 0
            property real clickY: 0

            onTriggered: {
                onScreenGimbalController.mouseClicked(clickX, clickY)
                cameraTrackingController.mouseClicked(clickX, clickY)
            }
        }

        onDoubleClicked: {
            // Fires on the second press of a double-click. The second release still emits
            // onReleased, so flag it to prevent re-arming the single-click timer.
            _doubleClicked = true
            singleClickTimer.stop()
            QGroundControl.videoManager.fullScreen = !QGroundControl.videoManager.fullScreen
        }

        onPressed: (mouse) => {
            _pressX = mouse.x
            _pressY = mouse.y
            _dragging = false
            // Clear any stale flag (e.g. double-click followed by drag releases through the
            // drag branch without consuming it). Safe: pressed is emitted before doubleClicked.
            _doubleClicked = false
        }

        onPositionChanged: (mouse) => {
            if (!_dragging && (Math.abs(mouse.x - _pressX) >= _dragThreshold || Math.abs(mouse.y - _pressY) >= _dragThreshold)) {
                _dragging = true
                onScreenGimbalController.mouseDragStart(_pressX, _pressY)
                cameraTrackingController.mouseDragStart(_pressX, _pressY)
            }
            if (_dragging) {
                onScreenGimbalController.mouseDragPositionChanged(mouse.x, mouse.y)
                cameraTrackingController.mouseDragPositionChanged(mouse.x, mouse.y)
            }
        }

        onReleased: (mouse) => {
            if (_dragging) {
                onScreenGimbalController.mouseDragEnd()
                cameraTrackingController.mouseDragEnd(mouse.x, mouse.y)
            } else if (_doubleClicked) {
                // Second release of a double-click - fullscreen toggle already handled
                _doubleClicked = false
            } else {
                singleClickTimer.clickX = mouse.x
                singleClickTimer.clickY = mouse.y
                singleClickTimer.restart()
            }
            _dragging = false
        }
    }

    ProximityRadarVideoView{
        anchors.fill:   parent
        vehicle:        QGroundControl.multiVehicleManager.activeVehicle
    }

    ObstacleDistanceOverlayVideo {
        id: obstacleDistance
        showText: pipState.state === pipState.fullState
    }

    //-- Additional cameras shown simultaneously as picture-in-picture tiles
    Item {
        id:             multiViewTiles
        anchors.fill:   parent
        visible:        videoStreaming.visible

        readonly property string _tileSizeSettingsKey: "VideoTileSize"
        property real _tileSize: ScreenTools.defaultFontPixelWidth * 34
        property real _minSize:  0.10
        property real _maxSize:  0.5

        Component.onCompleted: {
            var savedSize = parseFloat(QGroundControl.loadGlobalSetting(_tileSizeSettingsKey, "0"))
            if (savedSize > 0) {
                _tileSize = savedSize
            }
        }

        Repeater {
            model: QGroundControl.videoManager.maxVideoTiles()

            delegate: Rectangle {
                id:             tile
                property int cameraNumber: (QGroundControl.videoManager.activeVideoSource,
                                            QGroundControl.settingsManager.videoSettings.multiViewEnabled.rawValue,
                                            QGroundControl.videoManager.tileCameraNumber(index))
                width:          Math.min(Math.max(multiViewTiles._tileSize, multiViewTiles.width * multiViewTiles._minSize), multiViewTiles.width * multiViewTiles._maxSize)
                height:         Math.round(width * 9 / 16)
                visible:        cameraNumber > 0 && !QGroundControl.videoManager.fullScreen && pipState.state === pipState.fullState
                color:          tileExpanded ? "black" : "transparent"

                readonly property string _tileExpandedSettingsKey: "VideoTileCamera" + cameraNumber + "Expanded"
                property bool tileExpanded: QGroundControl.loadBoolGlobalSetting(_tileExpandedSettingsKey, true)

                on_TileExpandedSettingsKeyChanged: {
                    tileExpanded = QGroundControl.loadBoolGlobalSetting(_tileExpandedSettingsKey, true)
                    QGroundControl.videoManager.registerTileItem(index, tileExpanded ? tileVideo : null)
                }

                function setTileExpanded(expanded) {
                    QGroundControl.saveBoolGlobalSetting(_tileExpandedSettingsKey, expanded)
                    tileExpanded = expanded
                    QGroundControl.videoManager.registerTileItem(index, expanded ? tileVideo : null)
                }

                DragToPosition {
                    id:                 tileDragPosition
                    target:             tile
                    settingsKeyPrefix:  "VideoTile" + index
                    defaultX:           multiViewTiles.width - tile.width - ScreenTools.defaultFontPixelWidth * 2
                    defaultY:           (multiViewTiles.height - tile.height) / 2 + index * (tile.height + ScreenTools.defaultFontPixelHeight * 0.5)
                }

                DragHandler {
                    enabled: tile.tileExpanded
                    onActiveChanged: if (!active) tileDragPosition.commit()
                }

                VideoOutput {
                    id:                 tileVideo
                    objectName:         "extraVideo" + index
                    anchors.fill:       parent
                    visible:            tile.tileExpanded
                    fillMode:           VideoOutput.PreserveAspectFit
                    Component.onCompleted: QGroundControl.videoManager.registerTileItem(index, tile.tileExpanded ? this : null)
                }

                property string statusText: {
                    var statuses = QGroundControl.videoManager.cameraStatuses
                    var cam = tile.cameraNumber - 1
                    return cam >= 0 && cam < statuses.length ? statuses[cam] : ""
                }

                QGCLabel {
                    anchors.centerIn:   parent
                    text:               tile.statusText
                    color:              "white"
                    font.pointSize:     ScreenTools.smallFontPointSize
                    visible:            tile.statusText !== "" && tile.tileExpanded
                }

                Rectangle {
                    anchors.left:       parent.left
                    anchors.top:        parent.top
                    anchors.margins:    ScreenTools.defaultFontPixelHeight / 3
                    width:              tileLabel.contentWidth + ScreenTools.defaultFontPixelWidth * 2
                    height:             tileLabel.contentHeight + ScreenTools.defaultFontPixelHeight / 2
                    radius:             ScreenTools.defaultFontPixelHeight / 3
                    color:              Qt.rgba(0,0,0,0.75)
                    visible:            tileLabel.text !== "" && tile.tileExpanded

                    QGCLabel {
                        id:                 tileLabel
                        anchors.centerIn:   parent
                        text:               tile.cameraNumber > 0 ? QGroundControl.videoManager.cameraName(tile.cameraNumber - 1) : ""
                        color:              "white"
                        font.pointSize:     ScreenTools.smallFontPointSize
                    }
                }

                MouseArea {
                    id:             tileMouseArea
                    anchors.fill:   parent
                    enabled:        tile.tileExpanded
                    hoverEnabled:   true
                    onClicked:      QGroundControl.videoManager.promoteTile(index)
                }

                ResizeHandle {
                    id:          tileResizeHandle
                    target:      tile
                    enabled:     tile.tileExpanded
                    height:      ScreenTools.defaultFontPixelHeight * 2
                    iconVisible: tile.tileExpanded && (ScreenTools.isMobile || tileMouseArea.containsMouse || tileResizeHandle.pressed)
                    onResized:   (newWidth) => { multiViewTiles._tileSize = newWidth }
                    onCommitted: {
                        multiViewTiles._tileSize = tile.width
                        QGroundControl.saveGlobalSetting(multiViewTiles._tileSizeSettingsKey, tile.width.toString())
                        if (tileDragPosition.hasCustomPosition) {
                            tileDragPosition.commit()
                        } else {
                            tileDragPosition.rebind()
                        }
                    }
                }

                Image {
                    source:             "/qmlimages/pipHide.svg"
                    mipmap:             true
                    fillMode:           Image.PreserveAspectFit
                    anchors.left:       parent.left
                    anchors.bottom:     parent.bottom
                    visible:            tile.tileExpanded && (ScreenTools.isMobile || tileMouseArea.containsMouse)
                    height:             ScreenTools.defaultFontPixelHeight * 2
                    width:              height
                    sourceSize.height:  height
                    MouseArea {
                        anchors.fill:   parent
                        onClicked:      tile.setTileExpanded(false)
                    }
                }

                Rectangle {
                    anchors.left:       parent.left
                    anchors.bottom:     parent.bottom
                    height:             ScreenTools.defaultFontPixelHeight * 2
                    width:              height
                    radius:             ScreenTools.defaultFontPixelHeight / 3
                    color:              Qt.rgba(0,0,0,0.75)
                    visible:            !tile.tileExpanded
                    Image {
                        width:              parent.width * 0.75
                        height:             parent.height * 0.75
                        sourceSize.height:  height
                        source:             "/res/buttonRight.svg"
                        mipmap:             true
                        fillMode:           Image.PreserveAspectFit
                        anchors.centerIn:   parent
                    }
                    MouseArea {
                        anchors.fill:   parent
                        onClicked:      tile.setTileExpanded(true)
                    }
                }
            }
        }
    }
}
