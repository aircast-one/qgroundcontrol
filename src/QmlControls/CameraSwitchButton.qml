import QtQuick

import QGroundControl

import QGroundControl.Controls

Rectangle {
    id:         _root
    width:      contentRow.width + (_margin * 2)
    height:     contentRow.height + (ScreenTools.defaultFontPixelHeight * 0.5)
    radius:     ScreenTools.defaultFontPixelHeight / 3
    color:      "transparent"

    readonly property var _qgcPal: QGroundControl.globalPalette

    OverlaySurface {
        id:           glass
        anchors.fill: parent
        radius:       _root.radius
        highlight:    cameraMouseArea.containsMouse
    }

    property string text: ""
    signal clicked()

    property real _margin: ScreenTools.defaultFontPixelWidth

    Row {
        id:                 contentRow
        anchors.centerIn:   parent
        spacing:            ScreenTools.defaultFontPixelWidth * 0.5

        QGCColoredImage {
            anchors.verticalCenter: parent.verticalCenter
            height:                 cameraLabel.contentHeight
            width:                  height
            sourceSize.height:      height
            source:                 "/res/PlanSimpleItemCamera.svg"
            fillMode:               Image.PreserveAspectFit
            color:                  glass.contentColor
        }

        QGCLabel {
            id:                     cameraLabel
            anchors.verticalCenter: parent.verticalCenter
            text:                   _root.text
            color:                  glass.contentColor
            font.pointSize:         ScreenTools.smallFontPointSize
        }
    }

    MouseArea {
        id:                 cameraMouseArea
        anchors.fill:       parent
        hoverEnabled:       true
        preventStealing:    true
        cursorShape:        Qt.PointingHandCursor
        onClicked:          _root.clicked()
    }
}
