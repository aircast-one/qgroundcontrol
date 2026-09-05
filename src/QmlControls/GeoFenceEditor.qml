import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import QtPositioning

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls

// The fence, in the same grouped-list language as the mission items. This was nested tinted
// rectangles holding collapsible section headers over a three-column grid of checkboxes, radio
// buttons and "Del" buttons - a table of controls where the thing being described, the fence,
// never appeared as a row of its own.
QGCFlickable {
    id:             root
    contentHeight:  editorColumn.height
    clip:           true

    property var    myGeoFenceController
    property var    flightMap

    readonly property var  _qgcPal:         QGroundControl.globalPalette
    readonly property real _editFieldWidth: Math.min(width * 0.45, ScreenTools.defaultFontPixelWidth * 13)
    readonly property real _margin:         ScreenTools.defaultFontPixelWidth / 2
    readonly property bool _supported:      myGeoFenceController ? myGeoFenceController.supported : false
    readonly property var  _params:         (myGeoFenceController && myGeoFenceController.params) ? myGeoFenceController.params : []
    readonly property bool _hasFences:      myGeoFenceController ? (myGeoFenceController.polygons.count > 0 || myGeoFenceController.circles.count > 0) : false

    function _viewportCorners() {
        const rect = Qt.rect(flightMap.centerViewport.x, flightMap.centerViewport.y,
                             flightMap.centerViewport.width, flightMap.centerViewport.height)
        return [flightMap.toCoordinate(Qt.point(rect.x, rect.y), false /* clipToViewPort */),
                flightMap.toCoordinate(Qt.point(rect.x + rect.width, rect.y + rect.height), false /* clipToViewPort */)]
    }

    component SectionLabel: QGCLabel {
        leftPadding:    ScreenTools.defaultFontPixelHeight / 2
        font.pointSize: ScreenTools.smallFontPointSize
        font.bold:      true
        color:          Qt.alpha(root._qgcPal.text, 0.5)
    }

    Column {
        id:             editorColumn
        anchors.left:   parent.left
        anchors.right:  parent.right
        spacing:        ScreenTools.defaultFontPixelHeight * 0.5

        // Unsupported and empty are different states and used to read the same: a paragraph of
        // text with the whole form still sitting under it.
        PlanGroupCard {
            width:      parent.width
            visible:    !root._supported

            Column {
                width:      parent.width
                topPadding:    ScreenTools.defaultFontPixelHeight
                bottomPadding: ScreenTools.defaultFontPixelHeight
                spacing:       ScreenTools.defaultFontPixelHeight * 0.5

                QGCLabel {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text:                     qsTr("Not supported")
                    font.bold:                true
                }

                QGCLabel {
                    width:               parent.width - ScreenTools.defaultFontPixelWidth * 4
                    anchors.horizontalCenter: parent.horizontalCenter
                    horizontalAlignment: Text.AlignHCenter
                    wrapMode:            Text.WordWrap
                    font.pointSize:      ScreenTools.smallFontPointSize
                    color:               Qt.alpha(root._qgcPal.text, 0.5)
                    text:                qsTr("This vehicle does not support GeoFence.")
                }
            }
        }

        PlanGroupCard {
            width:      parent.width
            visible:    root._supported && !root._hasFences

            Column {
                width:         parent.width
                topPadding:    ScreenTools.defaultFontPixelHeight
                bottomPadding: ScreenTools.defaultFontPixelHeight
                spacing:       ScreenTools.defaultFontPixelHeight * 0.5

                QGCLabel {
                    anchors.horizontalCenter: parent.horizontalCenter
                    text:                     qsTr("No geofence")
                    font.bold:                true
                }

                QGCLabel {
                    width:                    parent.width - ScreenTools.defaultFontPixelWidth * 4
                    anchors.horizontalCenter: parent.horizontalCenter
                    horizontalAlignment:      Text.AlignHCenter
                    wrapMode:                 Text.WordWrap
                    font.pointSize:           ScreenTools.smallFontPointSize
                    color:                    Qt.alpha(root._qgcPal.text, 0.5)
                    text:                     qsTr("Keep the vehicle inside a boundary, or out of an area.")
                }
            }
        }

        SectionLabel {
            text:    qsTr("ADD FENCE")
            visible: root._supported
        }

        PlanGroupCard {
            width:      parent.width
            visible:    root._supported

            PlanGroupRow {
                objectName:  "fenceAddPolygon"
                text:        qsTr("＋  Polygon Fence")
                textColor:   root._qgcPal.primaryButton
                interactive: true
                onClicked: {
                    const corners = root._viewportCorners()
                    myGeoFenceController.addInclusionPolygon(corners[0], corners[1])
                }
            }

            PlanGroupRow {
                text:        qsTr("＋  Circular Fence")
                textColor:   root._qgcPal.primaryButton
                interactive: true
                onClicked: {
                    const corners = root._viewportCorners()
                    myGeoFenceController.addInclusionCircle(corners[0], corners[1])
                }
            }
        }

        SectionLabel {
            text:    qsTr("POLYGON FENCES")
            visible: root._supported && myGeoFenceController.polygons.count > 0
        }

        PlanGroupCard {
            width:      parent.width
            visible:    root._supported && myGeoFenceController.polygons.count > 0

            Repeater {
                model: myGeoFenceController.polygons

                PlanGroupRow {
                    text:        qsTr("Polygon %1").arg(index + 1)
                    description: object.inclusion ? qsTr("Inclusion") : qsTr("Exclusion")

                    QGCCheckBoxSlider {
                        anchors.verticalCenter: parent.verticalCenter
                        checked:                object.inclusion
                        onClicked:              object.inclusion = checked
                    }

                    FenceEditToggle {
                        anchors.verticalCenter: parent.verticalCenter
                        fenceObject:            object
                    }

                    FenceDeleteButton {
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked:              myGeoFenceController.deletePolygon(index)
                    }
                }
            }
        }

        SectionLabel {
            text:    qsTr("CIRCULAR FENCES")
            visible: root._supported && myGeoFenceController.circles.count > 0
        }

        PlanGroupCard {
            width:      parent.width
            visible:    root._supported && myGeoFenceController.circles.count > 0

            Repeater {
                model: myGeoFenceController.circles

                PlanGroupRow {
                    text:        qsTr("Circle %1").arg(index + 1)
                    description: object.inclusion ? qsTr("Inclusion") : qsTr("Exclusion")

                    FactTextField {
                        anchors.verticalCenter: parent.verticalCenter
                        width:                  ScreenTools.defaultFontPixelWidth * 8
                        fact:                   object.radius
                        showUnits:              true
                    }

                    FenceEditToggle {
                        anchors.verticalCenter: parent.verticalCenter
                        fenceObject:            object
                    }

                    FenceDeleteButton {
                        anchors.verticalCenter: parent.verticalCenter
                        onClicked:              myGeoFenceController.deleteCircle(index)
                    }
                }
            }
        }

        SectionLabel {
            text:    qsTr("SETTINGS")
            visible: root._supported && root._params.length > 0
        }

        PlanGroupCard {
            width:      parent.width
            visible:    root._supported && root._params.length > 0

            Repeater {
                model: root._params

                PlanGroupRow {
                    id:   paramRow
                    text: myGeoFenceController.paramLabels[index]

                    readonly property bool _showCombo: modelData.enumStrings.length > 0

                    FactTextField {
                        anchors.verticalCenter: parent.verticalCenter
                        width:                  root._editFieldWidth
                        showUnits:              true
                        fact:                   modelData
                        visible:                !paramRow._showCombo
                    }

                    FactComboBox {
                        anchors.verticalCenter: parent.verticalCenter
                        width:                  root._editFieldWidth
                        indexModel:             false
                        fact:                   paramRow._showCombo ? modelData : _nullFact
                        visible:                paramRow._showCombo

                        property var _nullFact: Fact { }
                    }
                }
            }
        }

        SectionLabel {
            text:    qsTr("BREACH RETURN POINT")
            visible: root._supported
        }

        PlanGroupCard {
            width:      parent.width
            visible:    root._supported

            PlanGroupRow {
                text:        qsTr("＋  Add Breach Return Point")
                textColor:   root._qgcPal.primaryButton
                interactive: true
                visible:     !myGeoFenceController.breachReturnPoint.isValid
                onClicked:   myGeoFenceController.breachReturnPoint = flightMap.center
            }

            PlanGroupRow {
                text:    qsTr("Altitude")
                visible: myGeoFenceController.breachReturnPoint.isValid

                FactTextField {
                    anchors.verticalCenter: parent.verticalCenter
                    width:                  root._editFieldWidth
                    showUnits:              true
                    fact:                   myGeoFenceController.breachReturnAltitude
                }
            }

            PlanGroupRow {
                text:        qsTr("Remove Breach Return Point")
                textColor:   root._qgcPal.colorRed
                interactive: true
                visible:     myGeoFenceController.breachReturnPoint.isValid
                onClicked:   myGeoFenceController.breachReturnPoint = QtPositioning.coordinate()
            }
        }
    }

    component FenceEditToggle: Rectangle {
        id:      editToggle
        width:   ScreenTools.defaultFontPixelHeight * 1.5
        height:  width
        radius:  width / 2
        color:   fenceObject.interactive ? Qt.alpha(root._qgcPal.primaryButton, 0.34) : Qt.alpha(root._qgcPal.text, 0.10)

        required property var fenceObject

        QGCColoredImage {
            anchors.centerIn:   parent
            source:             "/InstrumentValueIcons/edit-pencil.svg"
            height:             parent.height * 0.55
            width:              height
            sourceSize.height:  height
            fillMode:           Image.PreserveAspectFit
            mipmap:             true
            color:              editToggle.fenceObject.interactive ? root._qgcPal.primaryButton : root._qgcPal.text
        }

        QGCMouseArea {
            anchors.fill: parent
            onClicked: {
                const wasInteractive = editToggle.fenceObject.interactive
                myGeoFenceController.clearAllInteractive()
                editToggle.fenceObject.interactive = !wasInteractive
            }
        }
    }

    component FenceDeleteButton: Rectangle {
        id:     deleteButton
        width:  ScreenTools.defaultFontPixelHeight * 1.5
        height: width
        radius: width / 2
        color:  Qt.alpha(root._qgcPal.colorRed, deleteMouseArea.containsMouse ? 0.35 : 0.16)

        signal clicked()

        QGCLabel {
            anchors.centerIn: parent
            text:             "–"
            font.bold:        true
            color:            root._qgcPal.colorRed
        }

        QGCMouseArea {
            id:             deleteMouseArea
            anchors.fill:   parent
            hoverEnabled:   !ScreenTools.isMobile
            onClicked:      deleteButton.clicked()
        }
    }
}
