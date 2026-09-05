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
import QGroundControl.ScreenTools

Rectangle {
    id: _root

    property string icon: ""

    width:         ScreenTools.defaultFontPixelHeight * 1.2
    height:        width
    radius:        width / 2
    color:         "white"
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { }

    QGCColoredImage {
        anchors.centerIn:  parent
        visible:           _root.icon !== ""
        source:            _root.icon
        color:             Qt.rgba(0, 0, 0, 0.75)
        height:            parent.height * 0.6
        width:             height
        sourceSize.height: height
        fillMode:          Image.PreserveAspectFit
        mipmap:            true
    }
}
