import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

RowLayout {
    property string labelText: "Label"
    property string valueText: "value"
    property string valueColor: ""
    property bool   showDivider: true

    width:   parent.width
    height:  ScreenTools.defaultFontPixelHeight * 1.7
    spacing: ScreenTools.defaultFontPixelWidth

    QGCLabel {
        Layout.fillWidth: true
        text:             labelText.replace(/:$/, "")
        elide:            Text.ElideRight
    }

    QGCLabel {
        Layout.maximumWidth:    parent.width * 0.6
        text:                   valueText
        color:                  valueColor !== "" ? valueColor : Qt.alpha(QGroundControl.globalPalette.text, 0.55)
        elide:                  Text.ElideRight
        horizontalAlignment:    Text.AlignRight
    }
}
