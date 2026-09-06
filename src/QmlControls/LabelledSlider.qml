/****************************************************************************
 *
 * (c) 2009-2022 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

RowLayout {
    id: root

    signal moved(real value)

    property alias label:                   label.text
    property alias description:             descriptionLabel.text
    property alias from:                    slider.from
    property alias to:                      slider.to
    property alias value:                   slider.value
    property alias stepSize:                slider.stepSize
    property real  sliderPreferredWidth:    -1

    spacing: ScreenTools.defaultFontPixelWidth * 2

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

    QGCSlider {
        id:                     slider
        Layout.preferredWidth:  sliderPreferredWidth
        onMoved:                root.moved(value)
    }
}

