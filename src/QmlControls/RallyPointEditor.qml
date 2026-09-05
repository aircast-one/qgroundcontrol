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
import QGroundControl.FactControls
import QGroundControl.ScreenTools

// Rally points as one grouped list. This used to be a description card with an editor for
// whichever point happened to be current underneath it, so the set of rally points was only ever
// visible on the map, never in the panel.
QGCFlickable {
    id:             _root
    contentHeight:  rallyColumn.height
    clip:           true

    property var      controller     ///< RallyPointController
    property var      mapCenter      ///< Where a new point is dropped

    readonly property var _qgcPal: QGroundControl.globalPalette

Column {
    id:             rallyColumn
    anchors.left:   parent.left
    anchors.right:  parent.right
    spacing:        ScreenTools.defaultFontPixelHeight * 0.5

    PlanGroupCard {
        width:      parent.width
        visible:    !controller.supported || controller.points.count === 0

        Column {
            width:         parent.width
            topPadding:    ScreenTools.defaultFontPixelHeight
            bottomPadding: ScreenTools.defaultFontPixelHeight
            spacing:       ScreenTools.defaultFontPixelHeight * 0.5

            QGCLabel {
                anchors.horizontalCenter: parent.horizontalCenter
                font.bold:                true
                text:                     controller.supported ? qsTr("No rally points")
                                                                          : qsTr("Not supported")
            }

            QGCLabel {
                width:                    parent.width - ScreenTools.defaultFontPixelWidth * 4
                anchors.horizontalCenter: parent.horizontalCenter
                horizontalAlignment:      Text.AlignHCenter
                wrapMode:                 Text.WordWrap
                font.pointSize:           ScreenTools.smallFontPointSize
                color:                    Qt.alpha(_root._qgcPal.text, 0.5)
                text:                     controller.supported
                                              ? qsTr("Alternate landing points for Return to Launch. Turn on the waypoint tool and tap the map to place one.")
                                              : qsTr("This vehicle does not support Rally Points.")
            }
        }
    }

    QGCLabel {
        text:           qsTr("RALLY POINTS")
        leftPadding:    ScreenTools.defaultFontPixelHeight / 2
        font.pointSize: ScreenTools.smallFontPointSize
        font.bold:      true
        color:          Qt.alpha(_root._qgcPal.text, 0.5)
        visible:        controller.points.count > 0
    }

    PlanGroupCard {
        width:      parent.width
        visible:    controller.points.count > 0

        Repeater {
            model: controller.points

            Column {
                width: rallyColumn.width

                readonly property bool _isCurrent: object === controller.currentRallyPoint

                PlanGroupRow {
                    id:          rallyRow
                    text:        qsTr("Rally %1").arg(index + 1)
                    description: object.coordinate.latitude.toFixed(6) + ", " + object.coordinate.longitude.toFixed(6)
                    interactive: true
                    color:       parent._isCurrent ? Qt.alpha(_root._qgcPal.primaryButton, 0.16) : "transparent"
                    onClicked:   controller.currentRallyPoint = object

                    Rectangle {
                        anchors.verticalCenter: parent.verticalCenter
                        width:                  ScreenTools.defaultFontPixelHeight * 1.5
                        height:                 width
                        radius:                 width * 0.3
                        color:                  _root._qgcPal.colorGreen

                        QGCLabel {
                            anchors.centerIn:   parent
                            text:               index + 1
                            color:              "white"
                            font.bold:          true
                            font.pointSize:     ScreenTools.smallFontPointSize
                        }
                    }
                }

                Column {
                    width:          parent.width
                    visible:        parent._isCurrent
                    leftPadding:    ScreenTools.defaultFontPixelWidth * 1.5
                    rightPadding:   ScreenTools.defaultFontPixelWidth * 1.5
                    bottomPadding:  ScreenTools.defaultFontPixelHeight * 0.5
                    spacing:        ScreenTools.defaultFontPixelHeight * 0.3

                    Repeater {
                        model: object.textFieldFacts

                        Item {
                            width:  rallyColumn.width - ScreenTools.defaultFontPixelWidth * 3
                            height: rallyFactField.height

                            QGCLabel {
                                anchors.left:           parent.left
                                anchors.verticalCenter: parent.verticalCenter
                                text:                   modelData.name
                            }

                            FactTextField {
                                id:                     rallyFactField
                                anchors.right:          parent.right
                                width:                  Math.min(parent.width * 0.5, ScreenTools.defaultFontPixelWidth * 12)
                                showUnits:              true
                                fact:                   modelData
                            }
                        }
                    }

                    PlanGroupRow {
                        width:       rallyColumn.width - ScreenTools.defaultFontPixelWidth * 3
                        text:        qsTr("Delete Rally Point")
                        textColor:   _root._qgcPal.colorRed
                        interactive: true
                        onClicked:   controller.removePoint(object)
                    }
                }
            }
        }

        PlanGroupRow {
            text:        qsTr("＋  Add rally point")
            textColor:   _root._qgcPal.primaryButton
            interactive: true
            onClicked:   controller.addPoint(_root.mapCenter)
        }
    }

    PlanGroupCard {
        width:      parent.width
        visible:    controller.supported && controller.points.count === 0

        PlanGroupRow {
            text:        qsTr("＋  Add rally point")
            textColor:   _root._qgcPal.primaryButton
            interactive: true
            onClicked:   controller.addPoint(_root.mapCenter)
        }
    }
}
}
