import QtQuick
import QtQuick.Controls
import QGroundControl

Menu {
    id: _root

    property var _videoManager: QGroundControl.videoManager
    property var _airUnit:      QGroundControl.videoManager.airUnitCamera

    Component {
        id: cameraItemComponent
        MenuItem {
            required property int cameraIndex
            checkable:   true
            checked:     _root._videoManager.activeVideoSource === cameraIndex
            onTriggered: _root._videoManager.setActiveVideoSource(cameraIndex)
        }
    }

    Component {
        id: inputItemComponent
        MenuItem {
            required property int inputIndex
            checkable:   true
            checked:     _root._airUnit.activeInput === inputIndex
            onTriggered: _root._airUnit.selectInput(inputIndex)
        }
    }

    Component {
        id: separatorComponent
        MenuSeparator {}
    }

    function rebuild() {
        while (_root.count > 0) {
            _root.takeItem(0).destroy()
        }
        const cameras = _root._videoManager.videoSourceCount
        for (let i = 0; i < cameras; i++) {
            _root.addItem(cameraItemComponent.createObject(_root, { cameraIndex: i, text: _root._videoManager.cameraName(i) }))
        }
        if (_root._airUnit.available) {
            if (cameras > 1) {
                _root.addItem(separatorComponent.createObject(_root))
            }
            for (let k = 0; k < _root._airUnit.inputCount; k++) {
                _root.addItem(inputItemComponent.createObject(_root, { inputIndex: k, text: qsTr("Air unit %1").arg(_root._airUnit.inputName(k)) }))
            }
        }
    }

    function open(x, y) {
        rebuild()
        _root.popup(x, y)
    }
}
