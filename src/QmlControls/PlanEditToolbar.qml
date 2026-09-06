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

    property var    mapControl
    property bool   centered: false
    property string caption:  ""
    property var    tools:    []
    property bool   accentEnabled: true

    readonly property real availableWidth: mapControl.centerViewport.width

    x:      centered ? _viewportOrigin.x + (availableWidth - width) / 2
                     : _viewportOrigin.x + availableWidth - width
    y:      _viewportOrigin.y
    width:  Math.min(toolsRow.width + _pad * 2, availableWidth)
    lifted: true
    z:      QGroundControl.zOrderMapItems + 2

    readonly property point _viewportOrigin: mapControl.mapToItem(parent, mapControl.centerViewport.left,
                                                                          mapControl.centerViewport.top)

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
                objectName: "planEditCaption"
                visible:    _root.caption !== ""
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

                Item {
                    id:         toolItem
                    objectName: "planEditTool" + index

                    readonly property bool _separator: modelData.separator === true
                    readonly property bool _accent:    modelData.accent === true
                    readonly property bool _enabled:   !_accent || _root.accentEnabled

                    width:  _separator ? ScreenTools.defaultFontPixelWidth * 2
                                       : toolLabel.implicitWidth + ScreenTools.defaultFontPixelWidth * 3
                    height: parent.height

                    Rectangle {
                        anchors.centerIn: parent
                        visible:          toolItem._separator
                        width:            1
                        height:           parent.height * 0.55
                        color:            Qt.alpha(_root.contentColor, 0.2)
                    }

                    Rectangle {
                        anchors.fill: parent
                        visible:      !toolItem._separator
                        radius:       height / 2
                        color:        toolMouseArea.pressed       ? Qt.alpha(_root.contentColor, 0.18)
                                    : toolMouseArea.containsMouse ? Qt.alpha(_root.contentColor, 0.1)
                                                                  : "transparent"

                        Behavior on color { ColorAnimation { duration: 120 } }

                        QGCLabel {
                            id:               toolLabel
                            anchors.centerIn: parent
                            text:             toolItem._separator ? "" : modelData.text
                            font.bold:        toolItem._accent
                            color:            !toolItem._enabled ? Qt.alpha(_root.contentColor, 0.35)
                                            : toolItem._accent   ? _root._qgcPal.primaryButton
                                                                      : _root.contentColor
                        }

                        QGCMouseArea {
                            id:           toolMouseArea
                            anchors.fill: parent
                            enabled:      toolItem._enabled
                            hoverEnabled: !ScreenTools.isMobile
                            onClicked:    modelData.action()
                        }
                    }
                }
            }
        }
    }
}
