import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FactControls

RowLayout {
    property alias label:                   label.text
    property alias description:             descriptionLabel.text
    property alias fact:                    _comboBox.fact
    property alias indexModel:              _comboBox.indexModel
    property var   comboBox:                _comboBox
    property real  comboBoxPreferredWidth:  -1

    spacing: ScreenTools.defaultFontPixelWidth * 2

    signal activated(int index)

    ColumnLayout {
        Layout.fillWidth:   true
        spacing:            0

        QGCLabel {
            id:                 label
            Layout.fillWidth:   true
            elide:              Text.ElideRight
        }

        QGCLabel {
            id:                 descriptionLabel
            Layout.fillWidth:   true
            visible:            text !== ""
            wrapMode:           Text.WordWrap
            font.pointSize:     ScreenTools.smallFontPointSize
            color:              QGroundControl.globalPalette.colorGrey
        }
    }

    FactComboBox {
        id:                     _comboBox
        Layout.preferredWidth:  comboBoxPreferredWidth
        sizeToContents:         true
        plain:                  true

        onActivated: (index) => { parent.activated(index) }
    }
}
