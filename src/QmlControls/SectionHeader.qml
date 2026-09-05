import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import QGroundControl.ScreenTools
import QGroundControl.Palette

CheckBox {
    id:             control
    focusPolicy:    Qt.ClickFocus
    checked:        true
    leftPadding:    0

    property var            color:          qgcPal.colorGrey
    property bool           showSpacer:     true
    property ButtonGroup    buttonGroup:    null

    property real _sectionSpacer: ScreenTools.defaultFontPixelWidth / 2  // spacing between section headings

    onButtonGroupChanged: {
        if (buttonGroup) {
            buttonGroup.addButton(control)
        }
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    contentItem: ColumnLayout {
        Item {
            Layout.preferredHeight: control._sectionSpacer
            width:                  1
            visible:                control.showSpacer
        }

        // A section header names the group; it is not a heading competing with the rows under
        // it. Small, grey and quiet is the grouped-list treatment used everywhere else in the
        // app now.
        QGCLabel {
            text:               control.text
            color:              control.color
            font.pointSize:     ScreenTools.smallFontPointSize
            font.letterSpacing: 0.5
            Layout.fillWidth:   true

            QGCColoredImage {
                anchors.right:          parent.right
                anchors.verticalCenter: parent.verticalCenter
                width:                  parent.height / 2
                height:                 width
                source:                 "/qmlimages/arrow-down.png"
                color:                  qgcPal.text
                visible:                !control.checked
            }
        }

        // A hairline, not a bright white rule. At full text colour this was the loudest thing
        // in any panel it appeared in - louder than the controls it was introducing.
        Rectangle {
            Layout.fillWidth:   true
            height:             1
            color:              Qt.alpha(qgcPal.text, 0.15)
        }
    }

    indicator: Item {}
}
