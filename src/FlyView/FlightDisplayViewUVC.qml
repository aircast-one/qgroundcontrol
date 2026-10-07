import QtQuick
import QtQuick.Window
import QtMultimedia

import QGroundControl

Rectangle {
    id:                 _root
    width:              parent.width
    height:             parent.height
    implicitWidth:      videoOutput.implicitWidth
    implicitHeight:     videoOutput.implicitHeight
    color:              Qt.rgba(0,0,0,0.75)
    clip:               true
    anchors.centerIn:   parent
    visible:            _videoManager.isUvc

    property var _videoManager: QGroundControl.videoManager

    function adjustAspectRatio() {
        var resolution = camera.cameraFormat.resolution
        if (resolution.height > 0 && resolution.width > 0) {
            var aspectRatio = resolution.width / resolution.height
            _root.height = parent.height * aspectRatio
        }
    }

    MediaDevices {
        id: mediaDevices

        function findCameraDevice(cameraId) {
            var videoInputs = mediaDevices.videoInputs
            for (var i = 0; i < videoInputs.length; i++) {
                if (videoInputs[i].description === cameraId) {
                    return videoInputs[i]
                }
            }
            return mediaDevices.defaultVideoInput
        }
    }

    CaptureSession {
        camera: Camera {
            id:             camera
            cameraDevice:   mediaDevices.findCameraDevice(_videoManager.uvcVideoSourceID)
            active:         _videoManager.isUvc

            onCameraDeviceChanged: {
                if (active) {
                    adjustAspectRatio()
                }
            }

            onActiveChanged: {
                if (active) {
                    adjustAspectRatio()
                }
            }
        }
        videoOutput: videoOutput
    }

    VideoOutput {
        id:             videoOutput
        anchors.fill:   parent
        fillMode:       VideoOutput.PreserveAspectCrop
        orientation:    QGroundControl.isRadiomasterAx12 ? videoOutput._undoCameraRotation(Screen.orientation) : 0

        function _undoCameraRotation(screenOrientation) {
            switch (screenOrientation) {
            case Qt.LandscapeOrientation:
                return 90
            case Qt.InvertedLandscapeOrientation:
                return 270
            case Qt.InvertedPortraitOrientation:
                return 180
            default:
                return 0
            }
        }
    }
}
