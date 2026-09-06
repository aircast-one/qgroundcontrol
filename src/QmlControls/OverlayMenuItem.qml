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
    id:                 root
    Layout.fillWidth:   true
    implicitWidth:      _leftPadding + menuLabel.implicitWidth + _edgePadding * 2 +
                        (menuTrailing.visible ? menuTrailing.implicitWidth + _edgePadding : 0)
    implicitHeight:     Math.max(ScreenTools.defaultFontPixelHeight * 1.9,
                                 menuText.height + ScreenTools.defaultFontPixelHeight * 0.9)
    radius:             ScreenTools.defaultFontPixelHeight / 3
    color:              current                ? Qt.alpha(contentColor, 0.16)
                      : menuMouseArea.pressed  ? Qt.alpha(contentColor, 0.15)
                      : menuMouseArea.containsMouse ? Qt.alpha(contentColor, 0.08)
                                                    : "transparent"

    property alias  text:        menuLabel.text
    property alias  textColor:   menuLabel.color
    property alias  description: menuDescription.text
    property bool   checkable:   false
    property bool   reserveGutter: checkable
    property bool   checked:     false
    property bool   busy:        false
    property string icon:        ""
    property bool   current:     false
    property bool   disclosure:  false
    property string detail:      ""
    property color  contentColor: QGroundControl.globalPalette.text

    signal clicked()
    signal pressAndHold()

    readonly property color _accent:      contentColor
    readonly property real _edgePadding: ScreenTools.defaultFontPixelWidth * 1.5
    readonly property real _gutter:      ScreenTools.defaultFontPixelWidth * 2.5
    readonly property real _leftPadding: _edgePadding + ((reserveGutter || icon !== "") ? _gutter : 0)

    component GutterSlot: Item {
        anchors.left:           parent.left
        anchors.leftMargin:     root._edgePadding
        anchors.top:            menuText.top
        anchors.topMargin:      (menuLabel.height - height) / 2
        width:                  height
    }

    GutterSlot {
        height:     ScreenTools.defaultFontPixelHeight * 0.8
        visible:    root.checkable && root.checked && !root.busy

        QGCColoredImage {
            anchors.fill:       parent
            source:             "/InstrumentValueIcons/checkmark.svg"
            color:              root._accent
            sourceSize.height:  height
            fillMode:           Image.PreserveAspectFit
            mipmap:             true
        }
    }

    GutterSlot {
        height:     ScreenTools.defaultFontPixelHeight * 0.8
        visible:    root.busy

        OverlayActivityRing {
            anchors.fill:   parent
            contentColor:   root._accent
        }
    }

    GutterSlot {
        height:     ScreenTools.defaultFontPixelHeight
        visible:    root.icon !== ""

        QGCColoredImage {
            anchors.fill:       parent
            source:             root.icon
            color:              root._accent
            sourceSize.height:  height
            fillMode:           Image.PreserveAspectFit
            mipmap:             true
        }
    }

    Row {
        id:                     menuTrailing
        anchors.right:          parent.right
        anchors.rightMargin:    root._edgePadding
        anchors.verticalCenter: parent.verticalCenter
        spacing:                ScreenTools.defaultFontPixelWidth / 2
        visible:                root.disclosure || root.detail !== ""

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            text:                   root.detail
            color:                  Qt.rgba(root._accent.r, root._accent.g, root._accent.b, 0.5)
            visible:                text !== ""
        }

        QGCColoredImage {
            anchors.verticalCenter: parent.verticalCenter
            source:                 "/InstrumentValueIcons/cheveron-right.svg"
            color:                  Qt.rgba(root._accent.r, root._accent.g, root._accent.b, 0.5)
            height:                 ScreenTools.defaultFontPixelHeight * 0.9
            width:                  height
            sourceSize.height:      height
            fillMode:               Image.PreserveAspectFit
            mipmap:                 true
            visible:                root.disclosure
        }
    }

    Column {
        id:                     menuText
        anchors.left:           parent.left
        anchors.leftMargin:     root._leftPadding
        anchors.right:          menuTrailing.visible ? menuTrailing.left : parent.right
        anchors.rightMargin:    root._edgePadding
        anchors.verticalCenter: parent.verticalCenter
        spacing:                ScreenTools.defaultFontPixelHeight * 0.25

        QGCLabel {
            id:     menuLabel
            color:  root._accent
        }

        QGCLabel {
            id:             menuDescription
            width:          parent.width
            visible:        text !== ""
            wrapMode:       Text.WordWrap
            font.pointSize: ScreenTools.smallFontPointSize
            color:          Qt.rgba(root._accent.r, root._accent.g, root._accent.b, 0.65)
        }
    }

    QGCMouseArea {
        id:             menuMouseArea
        anchors.fill:   parent
        hoverEnabled:   !ScreenTools.isMobile
        onClicked:      root.clicked()
        onPressAndHold: root.pressAndHold()
    }
}
