import QtQuick

import QGroundControl
import QGroundControl.Controls

Item {
    id:             control
    width:          label.contentWidth
    anchors.top:    parent.top
    anchors.bottom: parent.bottom

    property bool showIndicator: _usbRadio !== null && _usbRadio.state !== "off"

    readonly property var _usbRadio: QGroundControl.usbRadio
    readonly property var _states: ({
        live:       qsTr("Radio · live"),
        ready:      qsTr("Radio · ready"),
        paused:     qsTr("Radio · paused"),
        standby:    qsTr("Radio · standby"),
        stopped:    qsTr("Radio · stopped")
    })

    QGCPalette { id: qgcPal }

    QGCLabel {
        id:                     label
        anchors.verticalCenter: parent.verticalCenter
        text:                   control._usbRadio ? (control._states[control._usbRadio.state] || "") : ""
        color:                  !control._usbRadio                      ? qgcPal.buttonText
                                : control._usbRadio.state === "stopped" ? qgcPal.colorRed
                                : control._usbRadio.state === "paused"  ? qgcPal.colorOrange
                                                                        : qgcPal.buttonText
    }

    QGCMouseArea {
        fillItem:   control
        onClicked:  mainWindow.showSettingsTool("Comm Links")
    }
}
