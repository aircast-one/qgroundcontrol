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
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.Controls

Rectangle {
    id:         _root
    color:      qgcPal.toolbarBackground
    width:      roundButtons ? _roundWidth : ScreenTools.defaultFontPixelWidth * 8
    height:     Math.min(maxHeight, toolStripColumn.height + (flickable.anchors.margins * 2))
    radius:     ScreenTools.defaultFontPixelWidth / 2

    property alias  model:              repeater.model
    property real   maxHeight
    property alias  title:              titleLabel.text
    property var    fontSize:           ScreenTools.smallFontPointSize

    property bool   roundButtons:       false

    readonly property real _roundWidth: repeater.count > 0 && repeater.itemAt(0)
                                            ? repeater.itemAt(0).discSize + (flickable.anchors.margins * 2)
                                            : ScreenTools.defaultFontPixelWidth * 8
    property bool   editing:            false
    property alias  buttonSpacing:      toolStripColumn.spacing

    signal held()

    readonly property color _discColor: qgcPal.overlayBackground

    property var _dropPanel: dropPanel

    function simulateClick(buttonIndex) {
        buttonIndex = buttonIndex + 1
        var button = toolStripColumn.children[buttonIndex]
        if (button.checkable) {
            button.checked = !button.checked
        }
        button.clicked()
    }

    signal dropped(int index)

    DeadMouseArea {
        anchors.fill: parent
    }

    QGCFlickable {
        id:                 flickable
        anchors.margins:    ScreenTools.defaultFontPixelWidth * 0.4
        anchors.top:        parent.top
        anchors.left:       parent.left
        anchors.right:      parent.right
        height:             parent.height - anchors.margins * 2
        contentHeight:      toolStripColumn.height
        flickableDirection: Flickable.VerticalFlick
        clip:               true

        Column {
            id:             toolStripColumn
            anchors.left:   parent.left
            anchors.right:  parent.right
            spacing:        ScreenTools.defaultFontPixelWidth * 0.25

            QGCLabel {
                id:                     titleLabel
                anchors.left:           parent.left
                anchors.right:          parent.right
                horizontalAlignment:    Text.AlignHCenter
                font.pointSize:         ScreenTools.smallFontPointSize
                visible:                title != ""
            }

            Repeater {
                id: repeater

                ToolStripHoverButton {
                    id:                 buttonTemplate
                    anchors.left:       toolStripColumn.left
                    anchors.right:      toolStripColumn.right
                    height:             _root.roundButtons ? implicitHeight : width
                    radius:             _root.roundButtons ? discSize / 2 : ScreenTools.defaultFontPixelWidth / 2
                    iconOnly:           _root.roundButtons
                    fontPointSize:      _root.fontSize
                    glass:              _root.roundButtons
                    bkColor:            _root.roundButtons ? _root._discColor : qgcPal.toolbarBackground
                    bkHoverColor:       _root.roundButtons ? Qt.rgba(_root._discColor.r, _root._discColor.g, _root._discColor.b, 1) : qgcPal.toolStripHoverColor
                    bkCheckedColor:     _root.roundButtons ? qgcPal.text : qgcPal.buttonHighlight
                    contentColor:       _root.roundButtons ? qgcPal.text : qgcPal.buttonText
                    contentCheckedColor: _root.roundButtons ? qgcPal.window : qgcPal.buttonHighlightText
                    borderColor:        _root.roundButtons ? qgcPal.overlayBorder : "transparent"
                    borderWidth:        _root.roundButtons ? 1 : 0
                    toolStripAction:    modelData
                    dropPanel:          _dropPanel
                    editing:            _root.editing
                    onDropped: (index) => _root.dropped(index)
                    onPressAndHold:     _root.held()

                    onCheckedChanged: {
                        if (checked) {
                            for (var i=0; i<repeater.count; i++) {
                                if (i != index) {
                                    var button = repeater.itemAt(i)
                                    if (button.checked) {
                                        button.checked = false
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    ToolStripDropPanel {
        id:         dropPanel
        toolStrip:  _root
    }
}
