
import QtQuick

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.FlightMap

Item {
    id:     _root
    height: editorColumn.height
    width:  availableWidth

    property real _cameraMinTriggerInterval: missionItem.cameraCalc.minTriggerInterval.rawValue

    readonly property real _gap:   ScreenTools.defaultFontPixelHeight * 0.7
    readonly property var  _units: QGroundControl.unitsConversion

    function polygonCaptureStarted() {
        missionItem.clearPolygon()
    }

    function polygonCaptureFinished(coordinates) {
        for (var i=0; i<coordinates.length; i++) {
            missionItem.addPolygonCoordinate(coordinates[i])
        }
    }

    function polygonAdjustVertex(vertexIndex, vertexCoordinate) {
        missionItem.adjustPolygonCoordinate(vertexIndex, vertexCoordinate)
    }

    function polygonAdjustStarted() { }
    function polygonAdjustFinished() { }

    function _vertical(meters) {
        return _units.formatMeasure(_units.metersToAppSettingsVerticalDistanceUnits(meters), _units.appSettingsVerticalDistanceUnitsString)
    }

    QGCPalette { id: qgcPal; colorGroupEnabled: true }

    Column {
        id:            editorColumn
        anchors.left:  parent.left
        anchors.right: parent.right
        spacing:       _root._gap

        PlanGroupCard {
            width:   parent.width
            visible: !missionItem.structurePolygon.isValid || missionItem.wizardMode

            Item {
                width:  parent.width
                height: helpLabel.implicitHeight + ScreenTools.defaultFontPixelHeight

                QGCLabel {
                    id:                     helpLabel
                    anchors.left:           parent.left
                    anchors.right:          parent.right
                    anchors.leftMargin:     ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.rightMargin:    ScreenTools.defaultFontPixelWidth * 1.5
                    anchors.verticalCenter: parent.verticalCenter
                    wrapMode:               Text.WordWrap
                    color:                  Qt.alpha(qgcPal.text, 0.7)
                    text:                   qsTr("Draw the structure outline with the Polygon Tools, at the top of the map.")
                }
            }
        }

        Column {
            width:   parent.width
            spacing: _root._gap
            visible: missionItem.structurePolygon.isValid && !missionItem.wizardMode

            OverlaySegmentedControl {
                id:          tabBar
                width:       parent.width
                segments:    [ qsTr("Grid"), qsTr("Camera") ]
                onActivated: (index) => currentIndex = index
            }

            Column {
                width:   parent.width
                spacing: _root._gap
                visible: tabBar.currentIndex === 0

                PlanGroupCard {
                    width:   parent.width
                    visible: missionItem.cameraShots > 0 && _cameraMinTriggerInterval !== 0 && _cameraMinTriggerInterval > missionItem.timeBetweenShots

                    PlanGroupRow {
                        text:        qsTr("Photo interval too short")
                        description: qsTr("The camera needs at least %1 s between photos").arg(_cameraMinTriggerInterval.toFixed(1))
                        textColor:   qgcPal.warningText
                    }
                }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        text:        qsTr("Camera")
                        value:       missionItem.cameraCalc.isManualCamera ? qsTr("Manual")
                                   : missionItem.cameraCalc.isCustomCamera ? qsTr("Custom")
                                                                           : missionItem.cameraCalc.cameraBrand + " " + missionItem.cameraCalc.cameraModel
                        showChevron: true
                        interactive: true
                        onClicked:   tabBar.currentIndex = 1
                    }
                }

                CameraCalcGrid {
                    width:                  parent.width
                    cameraCalc:             missionItem.cameraCalc
                    distanceToSurfaceLabel: qsTr("Scan distance")
                    frontalDistanceLabel:   qsTr("Layer height")
                    sideDistanceLabel:      qsTr("Trigger distance")
                }

                QGCLabel {
                    width:          parent.width
                    leftPadding:    ScreenTools.defaultFontPixelHeight / 2
                    rightPadding:   ScreenTools.defaultFontPixelHeight / 2
                    wrapMode:       Text.WordWrap
                    font.pointSize: ScreenTools.smallFontPointSize
                    color:          Qt.alpha(qgcPal.text, 0.5)
                    text:           qsTr("The polygon outlines the structure's surface, not the flight path.")
                }

                PlanSectionLabel { text: qsTr("SCAN") }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        text: qsTr("Start from")

                        OverlaySegmentedControl {
                            anchors.verticalCenter: parent.verticalCenter
                            width:                  ScreenTools.defaultFontPixelWidth * 20
                            height:                 ScreenTools.defaultFontPixelHeight * 1.8
                            segments:               [ qsTr("Bottom"), qsTr("Top") ]
                            currentIndex:           missionItem.startFromTop.value ? 1 : 0
                            onActivated:            (index) => missionItem.startFromTop.value = index
                        }
                    }

                    PlanFactRow {
                        text: qsTr("Structure height")
                        fact: missionItem.structureHeight
                    }

                    PlanFactRow {
                        text:         qsTr("Scan bottom altitude")
                        fact:         missionItem.scanBottomAlt
                        altitudeMode: QGroundControl.AltitudeModeRelative
                    }

                    PlanFactRow {
                        text:         qsTr("Entrance and exit altitude")
                        fact:         missionItem.entranceAlt
                        altitudeMode: QGroundControl.AltitudeModeRelative
                    }

                    PlanFactRow {
                        text:    qsTr("Gimbal pitch")
                        fact:    missionItem.gimbalPitch
                        visible: missionItem.cameraCalc.isManualCamera
                    }
                }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow {
                        text:        qsTr("Entry vertex")
                        value:       String(missionItem.entryVertex + 1)
                        showChevron: true
                        interactive: true
                        onClicked:   missionItem.rotateEntryPoint()
                    }
                }

                PlanSectionLabel { text: qsTr("STATISTICS") }

                PlanGroupCard {
                    width: parent.width

                    PlanGroupRow { text: qsTr("Layers");                value: missionItem.layers.valueString }
                    PlanGroupRow { text: qsTr("Layer height");          value: missionItem.cameraCalc.adjustedFootprintFrontal.valueString + " " + missionItem.cameraCalc.adjustedFootprintFrontal.units }
                    PlanGroupRow { text: qsTr("Top layer altitude");    value: _root._vertical(missionItem.topFlightAlt) }
                    PlanGroupRow { text: qsTr("Bottom layer altitude"); value: _root._vertical(missionItem.bottomFlightAlt) }
                    PlanGroupRow { text: qsTr("Photos");                value: String(missionItem.cameraShots) }
                    PlanGroupRow { text: qsTr("Photo interval");        value: qsTr("%1 s").arg(missionItem.timeBetweenShots.toFixed(1)) }
                }
            }

            CameraCalcCamera {
                width:      parent.width
                visible:    tabBar.currentIndex === 1
                cameraCalc: missionItem.cameraCalc
            }
        }
    }
}
