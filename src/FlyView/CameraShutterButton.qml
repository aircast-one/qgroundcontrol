import QtQuick

import QGroundControl
import QGroundControl.Controls

Rectangle {
    id:             _root
    width:          Math.max(ScreenTools.minTouchPixels * 1.2, ScreenTools.defaultFontPixelHeight * 3.2)
    height:         width
    radius:         width / 2
    color:          "transparent"
    border.color:   qgcPal.text
    border.width:   Math.max(3, width / 16)
    layer.enabled: true
    layer.effect:  OverlayShadowEffect { elevated: _root.lifted }

    OverlaySurface {
        anchors.fill:    parent
        anchors.margins: _root.border.width
        radius:          width / 2
    }

    property bool recording: false

    signal clicked()
    signal held()

    property bool editing:        false
    property bool lifted:         false
    property bool actionsEnabled: true

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    Rectangle {
        anchors.centerIn:   parent
        width:              recording ? parent.width * 0.42 : parent.width * 0.72
        height:             width
        radius:             recording ? ScreenTools.defaultFontPixelHeight / 4 : width / 2
        color:              qgcPal.colorRed
        opacity:            mouseArea.pressed ? 0.7 : 1

        Behavior on width  { NumberAnimation { duration: 120 } }
        Behavior on radius { NumberAnimation { duration: 120 } }
    }

    MouseArea {
        id:             mouseArea
        anchors.fill:   parent
        enabled:        !_root.editing
        onClicked:      if (_root.actionsEnabled) _root.clicked()
        onPressAndHold: _root.held()
    }
}
