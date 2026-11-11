import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

ColumnLayout {
    spacing: ScreenTools.defaultFontHeight / 2

    property var _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property int _boardBays: _activeVehicle ? _activeVehicle.dropBoardBays : 0
    property var _emptyBays: _activeVehicle ? _activeVehicle.dropBoardEmptyBays : []
    property var _usbRadio: QGroundControl.usbRadio
    property int _radioBay: _usbRadio && _usbRadio.receiving ? _usbRadio.selectedBay : 0
    property var _buttonTitles: [qsTr("Release"), qsTr("Grab"), qsTr("Hold")]
    property var _buttonActions: [MAVLinkEnums.GRIPPER_ACTION_RELEASE, MAVLinkEnums.GRIPPER_ACTION_GRAB, MAVLinkEnums.GRIPPER_ACTION_HOLD]

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

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
            property int  _bay:     index + 1
            property bool _empty:   _emptyBays.indexOf(_bay) >= 0
            property bool _picked:  _radioBay === _bay

            Layout.fillWidth:   true
            text:               qsTr("Drop %1").arg(_bay) + (_empty ? qsTr(" · empty") : "") + (_picked ? qsTr(" · 6POS") : "")
            opacity:            _empty ? 0.6 : 1
            borderWidth:        _picked ? 2 : (showBorder ? 1 : 0)
            borderColor:        _picked ? qgcPal.colorOrange : qgcPal.buttonBorder

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
