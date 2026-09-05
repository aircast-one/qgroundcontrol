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

// One row of an overlay menu: text on the panel, highlighted only while hovered or pressed.
// A row that can be the current choice reserves a leading gutter so labels stay aligned
// whether or not the checkmark is showing.
Rectangle {
    id:                 root
    Layout.fillWidth:   true
    implicitWidth:      _leftPadding + menuLabel.implicitWidth + ScreenTools.defaultFontPixelWidth * 3
    implicitHeight:     ScreenTools.defaultFontPixelHeight * 2.2
    radius:             ScreenTools.defaultFontPixelHeight / 3
    color:              current                ? Qt.rgba(1, 1, 1, 0.16)
                      : menuMouseArea.pressed  ? Qt.rgba(1, 1, 1, 0.15)
                      : menuMouseArea.containsMouse ? Qt.rgba(1, 1, 1, 0.08)
                                                    : "transparent"

    property alias  text:        menuLabel.text
    property alias  textColor:   menuLabel.color
    property bool   checkable:   false
    property bool   checked:     false
    property string icon:        ""
    property bool   current:     false
    property color  contentColor: QGroundControl.globalPalette.text

    signal clicked()

    readonly property color _accent:     contentColor
    readonly property real _gutter:      ScreenTools.defaultFontPixelWidth * 2.5
    readonly property real _leftPadding: (checkable || icon !== "") ? _gutter + ScreenTools.defaultFontPixelWidth
                                                                    : ScreenTools.defaultFontPixelWidth * 1.5

    QGCColoredImage {
        anchors.left:           parent.left
        anchors.leftMargin:     ScreenTools.defaultFontPixelWidth
        anchors.verticalCenter: parent.verticalCenter
        source:                 "/InstrumentValueIcons/checkmark.svg"
        color:                  QGroundControl.globalPalette.text
        height:                 ScreenTools.defaultFontPixelHeight * 0.8
        width:                  height
        sourceSize.height:      height
        fillMode:               Image.PreserveAspectFit
        mipmap:                 true
        visible:                root.checkable && root.checked
    }

    QGCColoredImage {
        anchors.left:           parent.left
        anchors.leftMargin:     ScreenTools.defaultFontPixelWidth
        anchors.verticalCenter: parent.verticalCenter
        source:                 root.icon
        color:                  root._accent
        height:                 ScreenTools.defaultFontPixelHeight
        width:                  height
        sourceSize.height:      height
        fillMode:               Image.PreserveAspectFit
        mipmap:                 true
        visible:                root.icon !== ""
    }

    QGCLabel {
        id:                     menuLabel
        anchors.left:           parent.left
        anchors.leftMargin:     root._leftPadding
        anchors.verticalCenter: parent.verticalCenter
        color:                  root._accent
    }

    QGCMouseArea {
        id:             menuMouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
        onClicked:      root.clicked()
    }
}
