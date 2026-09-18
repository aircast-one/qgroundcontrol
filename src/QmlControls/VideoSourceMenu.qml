import QtQuick
import QtQuick.Controls
import QGroundControl

Menu {
    id: _root

    property var _videoManager: QGroundControl.videoManager

    Component {
        id: cameraItemComponent
        MenuItem {
            required property int cameraIndex
            checkable:   true
            checked:     _root._videoManager.activeVideoSource === cameraIndex
            onTriggered: _root._videoManager.setActiveVideoSource(cameraIndex)
        }
    }

    function rebuild() {
        while (_root.count > 0) {
            _root.takeItem(0).destroy()
        }
        const cameras = _root._videoManager.videoSourceCount
        for (let i = 0; i < cameras; i++) {
            _root.addItem(cameraItemComponent.createObject(_root, { cameraIndex: i, text: _root._videoManager.cameraName(i) }))
        }
    }

    function open(x, y) {
        rebuild()
        _root.popup(x, y)
    }
}
