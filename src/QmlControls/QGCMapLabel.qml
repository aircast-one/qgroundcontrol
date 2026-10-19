import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls
QGCLabel {
    property var map
    color:      QGroundControl.globalPalette.text
    style:      Text.Outline
    styleColor: Qt.alpha(QGroundControl.globalPalette.window, 0.75)
}
