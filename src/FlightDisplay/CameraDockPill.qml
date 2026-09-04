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
import QGroundControl.Vehicle

OverlayCapsule {
    id:     _root
    width:  row.width + ScreenTools.defaultFontPixelWidth * 3
    height: ScreenTools.defaultFontPixelHeight * 2.2

    property var  dock
    property Item pipView

    readonly property var  _qgcPal:       QGroundControl.globalPalette
    readonly property var  _cameraManager: globals.activeVehicle ? globals.activeVehicle.cameraManager : null
    readonly property var  _camera:       _cameraManager && _cameraManager.cameras.count > 0 ? _cameraManager.cameras.get(_cameraManager.currentCamera) : null
    readonly property bool _thermal:      QGroundControl.videoManager.hasThermal && _camera !== null
    readonly property bool _thermalOn:    _camera ? _camera.thermalMode !== MavlinkCameraControl.THERMAL_OFF : false
    readonly property var  _numbers:      dock ? dock.cameraNumbers : []
    readonly property var  _states:       (QGroundControl.videoManager.cameraStatuses, QGroundControl.videoManager.cameraConnecting,
                                           _numbers.map((cameraNumber) => dock.stateOf(cameraNumber)))
    readonly property int  _live:         _states.filter((state) => state === "live").length
    readonly property int  _offline:      _states.filter((state) => state === "nosignal").length
    readonly property bool _hidden:       pipView ? !pipView.expanded : false
    readonly property string _focusName:  QGroundControl.videoManager.cameraName(QGroundControl.videoManager.activeVideoSource)

    function _dotColor(state) {
        return state === "live" ? _qgcPal.colorGreen : state === "connecting" ? _qgcPal.colorOrange : _qgcPal.colorRed
    }

    MouseArea {
        id:           pillMouseArea
        objectName:   "cameraDockPillToggle"
        anchors.fill: parent
        hoverEnabled: true
        cursorShape:  Qt.PointingHandCursor
        onClicked:    if (pipView) pipView.setExpanded(!pipView.expanded)
    }

    Row {
        id:               row
        anchors.centerIn: parent
        spacing:          ScreenTools.defaultFontPixelWidth

        Row {
            anchors.verticalCenter: parent.verticalCenter
            spacing:                ScreenTools.defaultFontPixelWidth * 0.4

            Repeater {
                model: _root._states

                Rectangle {
                    width:  ScreenTools.defaultFontPixelHeight * 0.45
                    height: width
                    radius: width / 2
                    color:  _root._dotColor(modelData)
                }
            }
        }

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            text:                   _root._hidden ? qsTr("%1 cameras hidden").arg(_root._numbers.length) : qsTr("%1 live").arg(_root._live)
            font.bold:              true
            font.pointSize:         ScreenTools.smallFontPointSize
        }

        QGCLabel {
            anchors.verticalCenter: parent.verticalCenter
            text:                   _root._hidden ? qsTr("tap to show") : qsTr("%1 offline · %2 in focus").arg(_root._offline).arg(_root._focusName)
            opacity:                0.65
            font.pointSize:         ScreenTools.smallFontPointSize
        }

        Rectangle {
            id:                     thermalSwitch
            objectName:             "cameraDockThermal"
            anchors.verticalCenter: parent.verticalCenter
            width:                  switchRow.width + 6
            height:                 switchRow.height + 6
            radius:                 height / 2
            color:                  Qt.rgba(0, 0, 0, 0.45)
            visible:                _root._thermal

            Row {
                id:               switchRow
                anchors.centerIn: parent

                Repeater {
                    model: [ { label: qsTr("Visible"), mode: MavlinkCameraControl.THERMAL_OFF, on: !_root._thermalOn },
                             { label: qsTr("Thermal"), mode: MavlinkCameraControl.THERMAL_FULL, on: _root._thermalOn } ]

                    Rectangle {
                        width:  segLabel.width + ScreenTools.defaultFontPixelWidth * 2
                        height: segLabel.height + ScreenTools.defaultFontPixelHeight * 0.4
                        radius: height / 2
                        color:  modelData.on ? Qt.rgba(1, 1, 1, 0.92) : "transparent"

                        QGCLabel {
                            id:               segLabel
                            anchors.centerIn: parent
                            text:             modelData.label
                            color:            modelData.on ? "black" : "white"
                            font.bold:        true
                            font.pointSize:   ScreenTools.smallFontPointSize
                        }

                        MouseArea {
                            anchors.fill: parent
                            cursorShape:  Qt.PointingHandCursor
                            onClicked:    if (_root._camera) _root._camera.thermalMode = modelData.mode
                        }
                    }
                }
            }
        }
    }
}
