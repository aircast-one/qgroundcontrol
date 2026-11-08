import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

ColumnLayout {
    spacing: ScreenTools.defaultFontHeight / 2

    property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property int _boardBays: _activeVehicle ? _activeVehicle.dropBoardBays : 0
    property var _buttonTitles: [qsTr("Release"), qsTr("Grab"), qsTr("Hold")]
    property var _buttonActions: [MAVLinkEnums.GRIPPER_ACTION_RELEASE, MAVLinkEnums.GRIPPER_ACTION_GRAB, MAVLinkEnums.GRIPPER_ACTION_HOLD]

    Repeater {
        model: _boardBays > 0 ? [] : _buttonTitles

        QGCDelayButton {
            Layout.fillWidth:   true
            text:               _buttonTitles[index]

            onActivated: {
                _activeVehicle.sendGripperAction(_buttonActions[index])
                dropPanel.hide()
            }
        }
    }

    Repeater {
        model: _boardBays

        QGCDelayButton {
            Layout.fillWidth:   true
            text:               qsTr("Drop %1").arg(index + 1)

            onActivated: {
                _activeVehicle.dropBay(index + 1)
                dropPanel.hide()
            }
        }
    }

    QGCDelayButton {
        Layout.fillWidth:   true
        visible:            _boardBays > 0
        text:               qsTr("Drop all")

        onActivated: {
            _activeVehicle.dropAllBays()
            dropPanel.hide()
        }
    }
}
