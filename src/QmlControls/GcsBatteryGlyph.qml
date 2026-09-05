/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.ScreenTools

Item {
    id: _root

    property real fill:     0
    property bool charging: false
    property color color:   QGroundControl.globalPalette.toolbarText

    height: ScreenTools.defaultFontPixelHeight * 1.15
    width:  _handheld ? body.width : base.width

    readonly property bool _handheld:    ScreenTools.isMobile
    readonly property real _clamped:     Math.max(0, Math.min(1, fill))
    readonly property real _stroke:      Math.max(1, Math.round(height * 0.07))
    readonly property real _bodyHeight:  _handheld ? height : Math.round(height * 0.74)

    Rectangle {
        id:                       body
        anchors.horizontalCenter: parent.horizontalCenter
        anchors.top:              parent.top
        width:                    Math.round(_bodyHeight * (_handheld ? 0.58 : 1.5))
        height:                   _bodyHeight
        radius:                   Math.round(_stroke * (_handheld ? 3 : 1.5))
        color:                    "transparent"
        border.color:             _root.color
        border.width:             _stroke

        Rectangle {
            anchors.left:           parent.left
            anchors.leftMargin:     _stroke * 2
            anchors.bottom:         parent.bottom
            anchors.bottomMargin:   _stroke * 2
            width:                  (parent.width - (_stroke * 4)) * (_handheld ? 1 : _root._clamped)
            height:                 (parent.height - (_stroke * 4)) * (_handheld ? _root._clamped : 1)
            radius:                 Math.max(1, Math.round(_stroke))
            color:                  _root.color

            Behavior on width  { NumberAnimation { duration: 300; easing.type: Easing.OutCubic } }
            Behavior on height { NumberAnimation { duration: 300; easing.type: Easing.OutCubic } }
        }
    }

    Rectangle {
        id:                       base
        anchors.top:              body.bottom
        anchors.topMargin:        Math.round(_root.height * 0.06)
        anchors.horizontalCenter: parent.horizontalCenter
        width:                    Math.round(body.width * 1.3)
        height:                   Math.max(1, Math.round(_root.height * 0.1))
        radius:                   height / 2
        color:                    _root.color
        visible:                  !_handheld
    }

    QGCColoredImage {
        anchors.centerIn:   body
        height:             Math.round(body.height * 0.7)
        width:              height
        source:             "/InstrumentValueIcons/bolt.svg"
        color:              QGroundControl.globalPalette.toolbarText
        visible:            _root.charging
    }
}
