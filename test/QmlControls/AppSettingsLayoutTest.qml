import QtQuick

import QGroundControl.Controls

Item {
    id:     root
    width:  428
    height: 903

    QtObject {
        id: mainWindow

        readonly property real width:  root.width
        readonly property real height: root.height
        property real panelRadius:            12
        property bool flyViewBackdropVisible: false

        function allowViewSwitch() { return true }
        function registerWindowDragExclusion(item) { }
        function showMessageDialog() { }
    }

    AppSettings {
        id:           settings
        objectName:   "settings"
        anchors.fill: parent
    }
}
