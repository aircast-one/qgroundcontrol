/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl.Controls
import QGroundControl.Palette
import QGroundControl.ScreenTools

Rectangle {
    id:         _root
    width:      Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * _widthInFontHeights)
    height:     width * aspect
    radius:     Math.min(width, height) / 2
    color:          checked ? qgcPal.text
                            : mouseArea.pressed ? Qt.alpha(qgcPal.overlayBackground, 1)
                                                : qgcPal.overlayBackground
    border.color:   checked ? "transparent"
                            : mouseArea.containsMouse ? Qt.alpha(qgcPal.text, _hoverBorderOpacity)
                                                      : qgcPal.overlayBorder
    border.width:   1
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { elevated: _root.lifted }

    property string icon:         ""
    property string text:         ""
    property bool   checked:      false
    property real   aspect:       1
    property real   iconRotation: 0

    signal clicked()
    signal held()

    property bool editing:        false
    property bool lifted:         false
    property bool actionsEnabled: true

    readonly property real _widthInFontHeights: 2.4
    readonly property real _glyphFraction:      0.5
    readonly property real _hoverBorderOpacity: 0.6

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    QGCColoredImage {
        anchors.centerIn:   parent
        source:             icon
        color:              checked ? qgcPal.window : qgcPal.text
        height:             Math.min(parent.width, parent.height) * _glyphFraction
        width:              height
        sourceSize.height:  height
        fillMode:           Image.PreserveAspectFit
        mipmap:             true
        smooth:             true
        rotation:           _root.iconRotation

        Behavior on rotation { NumberAnimation { duration: 320; easing.type: Easing.OutCubic } }
    }

    QGCLabel {
        anchors.centerIn:   parent
        width:              parent.width - ScreenTools.defaultFontPixelWidth
        visible:            icon === ""
        text:               _root.text
        color:              checked ? qgcPal.window : qgcPal.text
        font.pointSize:     ScreenTools.smallFontPointSize
        horizontalAlignment: Text.AlignHCenter
        elide:              Text.ElideRight
    }

    MouseArea {
        id:             mouseArea
        anchors.fill:   parent
        hoverEnabled:   true
        cursorShape:    Qt.PointingHandCursor
        enabled:        !_root.editing
        onClicked:      if (_root.actionsEnabled) _root.clicked()
        onPressAndHold: _root.held()
    }
}
