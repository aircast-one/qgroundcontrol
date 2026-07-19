/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Controllers
import QGroundControl.ScreenTools

Item {
    id: _root

    property Item pipView
    property Item pipState: videoPipState

    property int    _track_rec_x:       0
    property int    _track_rec_y:       0

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
        visible:        QGroundControl.videoManager.isStreamSource
    }
    //-- UVC Video (USB Camera or Video Device)
    Loader {
        id:             cameraLoader
        anchors.fill:   parent
        visible:        QGroundControl.videoManager.isUvc
        source:         QGroundControl.videoManager.uvcEnabled ? "qrc:/qml/QGroundControl/FlightDisplay/FlightDisplayViewUVC.qml" : "qrc:/qml/QGroundControl/FlightDisplay//FlightDisplayViewDummy.qml"
    }

    QGCLabel {
        text: qsTr("Double-click to exit full screen")
        font.pointSize: ScreenTools.largeFontPointSize
        visible: QGroundControl.videoManager.fullScreen && flyViewVideoMouseArea.containsMouse
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
        screenX:                 flyViewVideoMouseArea.mouseX
        screenY:                 flyViewVideoMouseArea.mouseY
        cameraTrackingEnabled:   videoStreaming._camera && videoStreaming._camera.trackingEnabled
    }

    MouseArea {
        id:                         flyViewVideoMouseArea
        anchors.fill:               parent
        enabled:                    pipState.state === pipState.fullState
        hoverEnabled:               true

        property double x0:         0
        property double x1:         0
        property double y0:         0
        property double y1:         0
        property double offset_x:   0
        property double offset_y:   0
        property double radius:     20
        property var trackingROI:   null
        property var trackingStatus: trackingStatusComponent.createObject(flyViewVideoMouseArea, {})

        onClicked:       onScreenGimbalController.clickControl()
        onDoubleClicked: QGroundControl.videoManager.fullScreen = !QGroundControl.videoManager.fullScreen

        onPressed:(mouse) => {
            onScreenGimbalController.pressControl()

            _track_rec_x = mouse.x
            _track_rec_y = mouse.y

            //create a new rectangle at the wanted position
            if(videoStreaming._camera) {
                if (videoStreaming._camera.trackingEnabled) {
                    trackingROI = trackingROIComponent.createObject(flyViewVideoMouseArea, {
                        "x": mouse.x,
                        "y": mouse.y
                    });
                }
            }
        }
        onPositionChanged: (mouse) => {
            //on move, update the width of rectangle
            if (trackingROI !== null) {
                if (mouse.x < trackingROI.x) {
                    trackingROI.x = mouse.x
                    trackingROI.width = Math.abs(mouse.x - _track_rec_x)
                } else {
                    trackingROI.width = Math.abs(mouse.x - trackingROI.x)
                }
                if (mouse.y < trackingROI.y) {
                    trackingROI.y = mouse.y
                    trackingROI.height = Math.abs(mouse.y - _track_rec_y)
                } else {
                    trackingROI.height = Math.abs(mouse.y - trackingROI.y)
                }
            }
        }
        onReleased: (mouse) => {
            onScreenGimbalController.releaseControl()
            
            //if there is already a selection, delete it
            if (trackingROI !== null) {
                trackingROI.destroy();
            }

            if(videoStreaming._camera) {
                if (videoStreaming._camera.trackingEnabled) {
                    // order coordinates --> top/left and bottom/right
                    x0 = Math.min(_track_rec_x, mouse.x)
                    x1 = Math.max(_track_rec_x, mouse.x)
                    y0 = Math.min(_track_rec_y, mouse.y)
                    y1 = Math.max(_track_rec_y, mouse.y)

                    //calculate offset between video stream rect and background (black stripes)
                    offset_x = (parent.width - videoStreaming.getWidth()) / 2
                    offset_y = (parent.height - videoStreaming.getHeight()) / 2

                    //convert absolute coords in background to absolute video stream coords
                    x0 = x0 - offset_x
                    x1 = x1 - offset_x
                    y0 = y0 - offset_y
                    y1 = y1 - offset_y

                    //convert absolute to relative coordinates and limit range to 0...1
                    x0 = Math.max(Math.min(x0 / videoStreaming.getWidth(), 1.0), 0.0)
                    x1 = Math.max(Math.min(x1 / videoStreaming.getWidth(), 1.0), 0.0)
                    y0 = Math.max(Math.min(y0 / videoStreaming.getHeight(), 1.0), 0.0)
                    y1 = Math.max(Math.min(y1 / videoStreaming.getHeight(), 1.0), 0.0)

                    //use point message if rectangle is very small
                    if (Math.abs(_track_rec_x - mouse.x) < 10 && Math.abs(_track_rec_y - mouse.y) < 10) {
                        var pt  = Qt.point(x0, y0)
                        videoStreaming._camera.startTracking(pt, radius / videoStreaming.getWidth())
                    } else {
                        var rec = Qt.rect(x0, y0, x1 - x0, y1 - y0)
                        videoStreaming._camera.startTracking(rec)
                    }
                    _track_rec_x = 0
                    _track_rec_y = 0
                }
            }
        }

        Component {
            id: trackingROIComponent

            Rectangle {
                color:              Qt.rgba(0.1,0.85,0.1,0.25)
                border.color:       "green"
                border.width:       1
            }
        }

        Component {
            id: trackingStatusComponent

            Rectangle {
                color:              "transparent"
                border.color:       "red"
                border.width:       5
                radius:             5
            }
        }

        Timer {
            id: trackingStatusTimer
            interval:               50
            repeat:                 true
            running:                true
            onTriggered: {
                if (videoStreaming._camera) {
                    if (videoStreaming._camera.trackingEnabled && videoStreaming._camera.trackingImageStatus) {
                        var margin_hor = (parent.parent.width - videoStreaming.getWidth()) / 2
                        var margin_ver = (parent.parent.height - videoStreaming.getHeight()) / 2
                        var left = margin_hor + videoStreaming.getWidth() * videoStreaming._camera.trackingImageRect.left
                        var top = margin_ver + videoStreaming.getHeight() * videoStreaming._camera.trackingImageRect.top
                        var right = margin_hor + videoStreaming.getWidth() * videoStreaming._camera.trackingImageRect.right
                        var bottom = margin_ver + !isNaN(videoStreaming._camera.trackingImageRect.bottom) ? videoStreaming.getHeight() * videoStreaming._camera.trackingImageRect.bottom : top + (right - left)
                        var width = right - left
                        var height = bottom - top

                        flyViewVideoMouseArea.trackingStatus.x = left
                        flyViewVideoMouseArea.trackingStatus.y = top
                        flyViewVideoMouseArea.trackingStatus.width = width
                        flyViewVideoMouseArea.trackingStatus.height = height
                    } else {
                        flyViewVideoMouseArea.trackingStatus.x = 0
                        flyViewVideoMouseArea.trackingStatus.y = 0
                        flyViewVideoMouseArea.trackingStatus.width = 0
                        flyViewVideoMouseArea.trackingStatus.height = 0
                    }
                }
            }
        }
    }

    //-- Note: the camera-switch button for the full-screen video is provided by FlyView at a
    //   higher z so it is not hidden behind the instrument overlays. The small-pip button is
    //   provided by PipView.

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
        property real _minSize:  0.10               // Percentage of parent control size
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
                // Reading the notifying properties forces re-evaluation when the active
                // source, camera list or multi-view toggle changes.
                property int cameraNumber: (QGroundControl.videoManager.activeVideoSource,
                                            QGroundControl.settingsManager.videoSettings.multiViewEnabled.rawValue,
                                            QGroundControl.videoManager.tileCameraNumber(index))
                width:          Math.min(Math.max(multiViewTiles._tileSize, multiViewTiles.width * multiViewTiles._minSize), multiViewTiles.width * multiViewTiles._maxSize)
                height:         Math.round(width * 9 / 16)
                visible:        cameraNumber > 0 && !QGroundControl.videoManager.fullScreen && pipState.state === pipState.fullState
                color:          tileExpanded ? "black" : "transparent"

                // Keyed by camera, not slot: collapsing a camera keeps it collapsed as it
                // moves between tile slots when the active camera changes.
                readonly property string _tileExpandedSettingsKey: "VideoTileCamera" + cameraNumber + "Expanded"
                property bool tileExpanded: QGroundControl.loadBoolGlobalSetting(_tileExpandedSettingsKey, true)

                on_TileExpandedSettingsKeyChanged: {
                    tileExpanded = QGroundControl.loadBoolGlobalSetting(_tileExpandedSettingsKey, true)
                    QGroundControl.videoManager.registerTileItem(index, tileExpanded ? tileVideo : null)
                }

                function setTileExpanded(expanded) {
                    QGroundControl.saveBoolGlobalSetting(_tileExpandedSettingsKey, expanded)
                    tileExpanded = expanded
                    // A collapsed tile hands its widget back so the sink stops feeding an
                    // invisible item; the stream connection itself stays warm.
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

                QGCVideoBackground {
                    id:                 tileVideo
                    objectName:         "extraVideo" + index  // must match VideoManager::_tileReceiverName
                    anchors.fill:       parent
                    visible:            tile.tileExpanded
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
                        // A resize alone must not turn the default stacking into a custom
                        // position; only update the stored spot if the user already dragged.
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
