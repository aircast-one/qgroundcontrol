import QtQuick

import QGroundControl
import QGroundControl.Controls

QGCLabel {
    anchors.horizontalCenter:   parent.horizontalCenter
    font.pointSize:             ScreenTools.smallFontPointSize
    style:                      Text.Outline
    color:                      QGroundControl.globalPalette.text
    styleColor:                 QGroundControl.globalPalette.window
}
