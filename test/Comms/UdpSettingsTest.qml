import QtQuick

import QGroundControl
import QGroundControl.Controls

Item {
    width:  600
    height: 600

    Loader {
        id:             loader
        objectName:     "udpSettingsLoader"
        anchors.fill:   parent
        source:         "qrc:/qml/QGroundControl/AppSettings/UdpSettings.qml"

        property var subEditConfig:      udpConfig
        property int _firstColumnWidth:  100
        property int _secondColumnWidth: 200
        property int _rowSpacing:        4
        property int _colSpacing:        8
    }
}
