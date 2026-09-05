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

OverlayCapsule {
    id: _root

    property real   availableWidth
    property string caption: ""
    property var    tools:   []

    width:  Math.min(toolsRow.width + _pad * 2, availableWidth)
    lifted: true
    z:      QGroundControl.zOrderMapItems + 2

    readonly property var  _qgcPal: QGroundControl.globalPalette
    readonly property real _pad:    ScreenTools.defaultFontPixelHeight * 0.25

    QGCFlickable {
        anchors.fill:       parent
        anchors.margins:    _root._pad
        clip:               true
        flickableDirection: Flickable.HorizontalFlick
        contentWidth:       toolsRow.width

        Row {
            id:     toolsRow
            height: _root.height - _root._pad * 2

            Item {
                visible: _root.caption !== ""
                width:   captionLabel.implicitWidth + ScreenTools.defaultFontPixelWidth * 3
                height:  parent.height

                QGCLabel {
                    id:               captionLabel
                    anchors.centerIn: parent
                    text:             _root.caption
                    color:            Qt.alpha(_root.contentColor, 0.65)
                }

                Rectangle {
                    anchors.right:          parent.right
                    anchors.verticalCenter: parent.verticalCenter
                    width:                  1
                    height:                 parent.height * 0.55
                    color:                  Qt.alpha(_root.contentColor, 0.2)
                }
            }

            Repeater {
                model: _root.tools

                Rectangle {
                    visible: modelData.visible !== false
                    width:   toolLabel.implicitWidth + ScreenTools.defaultFontPixelWidth * 3
                    height:  parent.height
                    radius:  height / 2
                    color:   toolMouseArea.pressed       ? Qt.alpha(_root.contentColor, 0.18)
                           : toolMouseArea.containsMouse ? Qt.alpha(_root.contentColor, 0.1)
                                                         : "transparent"

                    Behavior on color { ColorAnimation { duration: 120 } }

                    QGCLabel {
                        id:               toolLabel
                        anchors.centerIn: parent
                        text:             modelData.text
                        font.bold:        modelData.accent === true
                        color:            modelData.accent === true ? _root._qgcPal.primaryButton : _root.contentColor
                    }

                    QGCMouseArea {
                        id:           toolMouseArea
                        anchors.fill: parent
                        hoverEnabled: !ScreenTools.isMobile
                        onClicked:    modelData.action()
                    }
                }
            }
        }
    }
}
