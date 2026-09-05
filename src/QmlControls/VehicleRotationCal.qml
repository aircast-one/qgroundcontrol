/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtQuick.Controls

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

Rectangle {
    id: _root

    property bool   calValid:           false
    property bool   calInProgress:      false
    property string calInProgressText:  qsTr("Hold Still")
    property var    imageSource:        ""

    readonly property var   _qgcPal:     QGroundControl.globalPalette
    readonly property color _stateColor: calInProgress ? _qgcPal.colorBlue
                                       : calValid      ? _qgcPal.colorGreen
                                                       : Qt.alpha(_qgcPal.text, 0.12)

    radius:         ScreenTools.defaultFontPixelHeight * 0.8
    color:          Qt.alpha(_qgcPal.text, 0.055)
    border.width:   2
    border.color:   _stateColor

    Behavior on border.color { ColorAnimation { duration: 150 } }

    Image {
        anchors.fill:           parent
        anchors.margins:        ScreenTools.defaultFontPixelHeight * 0.5
        anchors.bottomMargin:   stateLabel.height + ScreenTools.defaultFontPixelHeight * 0.5
        source:                 imageSource
        fillMode:               Image.PreserveAspectFit
        smooth:                 true
    }

    QGCLabel {
        id:                     stateLabel
        anchors.left:           parent.left
        anchors.right:          parent.right
        anchors.bottom:         parent.bottom
        anchors.bottomMargin:   ScreenTools.defaultFontPixelHeight * 0.4
        horizontalAlignment:    Text.AlignHCenter
        font.bold:              calInProgress
        color:                  calInProgress || calValid ? _stateColor : Qt.alpha(_qgcPal.text, 0.5)
        text:                   calInProgress ? calInProgressText : (calValid ? qsTr("Done") : qsTr("Pending"))
    }
}
