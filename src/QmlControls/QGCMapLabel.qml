import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls
import QGroundControl.Palette

/// Text control used for displaying text of Maps
QGCLabel {
    color:      QGroundControl.globalPalette.overlayInk
    style:      Text.Outline
    styleColor: Qt.alpha(QGroundControl.globalPalette.overlayInkInverse, 0.75)
}
