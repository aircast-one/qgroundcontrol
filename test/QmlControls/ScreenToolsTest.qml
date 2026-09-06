import QtQuick

import QGroundControl.ScreenTools
import QGroundControl.ScreenToolsController

Item {
    id:     root
    width:  300
    height: 200

    readonly property real basePointSize:  ScreenTools.defaultFontPointSize
    readonly property real smallPointSize: ScreenTools.smallFontPointSize
    readonly property real minTouchPixels: ScreenTools.minTouchPixels
    readonly property real systemFontScale: ScreenToolsController.systemFontScale

    function setBasePointSize(pointSize) {
        ScreenTools._setBasePointSize(pointSize)
    }
}
