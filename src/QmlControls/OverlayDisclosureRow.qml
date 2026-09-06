/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.ScreenTools

Rectangle {
    id:                     root
    Layout.fillWidth:       true
    Layout.preferredHeight: ScreenTools.defaultFontPixelHeight * 1.9
    radius:                 ScreenTools.defaultFontPixelHeight / 3
    color:                  disclosureMouseArea.containsMouse ? Qt.alpha(QGroundControl.globalPalette.text, 0.08)
                                                              : "transparent"

    property alias text: disclosureLabel.text

    signal clicked()

    QGCLabel {
        id:                     disclosureLabel
        anchors.left:           parent.left
        anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
        anchors.right:          parent.right
        anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
        anchors.verticalCenter: parent.verticalCenter
        elide:                  Text.ElideRight
        color:                  QGroundControl.globalPalette.colorBlue
    }

    QGCMouseArea {
        id:             disclosureMouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
        onClicked:      root.clicked()
    }
}
