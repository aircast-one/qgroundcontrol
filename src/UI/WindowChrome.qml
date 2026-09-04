import QtQuick
import QtQuick.Controls
import QtQuick.Window

import QGroundControl
import QGroundControl.Palette
import QGroundControl.ScreenTools

import QWindowKit

Item {
    id: _root

    readonly property bool _isMac:      Qt.platform.os === "osx"
    readonly property bool _fullScreen: mainWindow.visibility === Window.FullScreen
    readonly property real leftInset:   _isMac && !_fullScreen ? macButtonArea.width : 0
    readonly property real rightInset:  !_isMac && !_fullScreen ? systemButtonRow.width : 0

    function excludeFromDrag(item) {
        windowAgent.setHitTestVisible(item, true)
    }

    Component.onCompleted: {
        windowAgent.setup(mainWindow)
        windowAgent.setTitleBar(dragArea)
        if (_isMac) {
            windowAgent.setSystemButtonArea(macButtonArea)
        } else {
            windowAgent.setSystemButton(WindowAgent.Minimize, minButton)
            windowAgent.setSystemButton(WindowAgent.Maximize, maxButton)
            windowAgent.setSystemButton(WindowAgent.Close, closeButton)
        }
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    WindowAgent { id: windowAgent }

    Item {
        id:             dragArea
        anchors.top:    parent.top
        anchors.left:   parent.left
        anchors.right:  parent.right
        height:         ScreenTools.toolbarHeight
    }

    Item {
        id:             macButtonArea
        anchors.top:    parent.top
        anchors.left:   parent.left
        width:          ScreenTools.defaultFontPixelWidth * 10
        height:         dragArea.height
        visible:        _root._isMac
    }

    component SystemButton: Button {
        id:             sysButton
        width:          ScreenTools.defaultFontPixelWidth * 5.5
        height:         dragArea.height
        leftPadding:    0
        topPadding:     0
        rightPadding:   0
        bottomPadding:  0
        leftInset:      0
        topInset:       0
        rightInset:     0
        bottomInset:    0

        property color  hoverColor:      Qt.rgba(1, 1, 1, 0.12)
        property color  hoverGlyphColor: qgcPal.text
        property color  glyphColor:      sysButton.pressed || sysButton.hovered ? sysButton.hoverGlyphColor : qgcPal.text

        background: Rectangle {
            color: sysButton.pressed || sysButton.hovered ? sysButton.hoverColor : "transparent"
        }
    }

    Row {
        id:             systemButtonRow
        anchors.top:    parent.top
        anchors.right:  parent.right
        visible:        !_root._isMac && !_root._fullScreen

        SystemButton {
            id:         minButton
            onClicked:  mainWindow.showMinimized()

            contentItem: Item {
                Rectangle {
                    anchors.centerIn:   parent
                    width:              ScreenTools.defaultFontPixelWidth * 1.75
                    height:             1
                    color:              minButton.glyphColor
                }
            }
        }

        SystemButton {
            id:         maxButton
            onClicked:  mainWindow.visibility === Window.Maximized ? mainWindow.showNormal() : mainWindow.showMaximized()

            contentItem: Item {
                Rectangle {
                    anchors.centerIn:   parent
                    width:              ScreenTools.defaultFontPixelWidth * 1.75
                    height:             width
                    radius:             mainWindow.visibility === Window.Maximized ? 2 : 0
                    color:              "transparent"
                    border.width:       1
                    border.color:       maxButton.glyphColor
                }
            }
        }

        SystemButton {
            id:                 closeButton
            hoverColor:         "#e81123"
            hoverGlyphColor:    "white"
            onClicked:          mainWindow.close()

            contentItem: Item {
                Rectangle {
                    anchors.centerIn:   parent
                    width:              ScreenTools.defaultFontPixelWidth * 2.25
                    height:             1
                    rotation:           45
                    antialiasing:       true
                    color:              closeButton.glyphColor
                }
                Rectangle {
                    anchors.centerIn:   parent
                    width:              ScreenTools.defaultFontPixelWidth * 2.25
                    height:             1
                    rotation:           -45
                    antialiasing:       true
                    color:              closeButton.glyphColor
                }
            }
        }
    }
}
