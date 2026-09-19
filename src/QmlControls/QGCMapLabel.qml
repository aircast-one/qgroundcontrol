import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls
/// Text control used for displaying text of Maps
QGCLabel {
    property var map
    color:      QGroundControl.globalPalette.overlayInk
    style:      Text.Outline
    styleColor: Qt.alpha(QGroundControl.globalPalette.overlayInkInverse, 0.75)
}
