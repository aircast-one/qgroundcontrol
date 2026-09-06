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
import QtQuick.Layouts

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.Controls
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager
import QGroundControl.Palette
import QGroundControl.AutoPilotPlugins.PX4
import QGroundControl.AutoPilotPlugins.APM

Item {
    id:             _summaryRoot
    objectName:     "vehicleSummary"
    anchors.fill:   parent

    readonly property var  _vehicle:        QGroundControl.multiVehicleManager.activeVehicle
    readonly property var  _plugin:         _vehicle ? _vehicle.autopilotPlugin : null
    readonly property var  _components:     _plugin ? _plugin.vehicleComponents : []
    readonly property bool _setupComplete:  _plugin ? _plugin.setupComplete : false
    readonly property real _fh:             ScreenTools.defaultFontPixelHeight
    readonly property real _fw:             ScreenTools.defaultFontPixelWidth
    readonly property real _pad:            _fw * 1.5
    readonly property real _radius:         _fh * 0.9
    readonly property real _minCardWidth:   ScreenTools.isTinyScreen ? _fw * 28 : _fw * 36
    readonly property real _cardSpacing:    _fw * 1.5
    readonly property int  _columns:        Math.max(1, Math.floor((width + _cardSpacing) / (_minCardWidth + _cardSpacing)))
    readonly property real _cardWidth:      (width - _cardSpacing * (_columns - 1)) / _columns
    readonly property string _firmwareText: _vehicle ? _vehicle.firmwareTypeString + " " + _vehicle.firmwareMajorVersion + "." + _vehicle.firmwareMinorVersion + "." + _vehicle.firmwarePatchVersion + " " + _vehicle.firmwareVersionTypeString : ""

    QGCPalette { id: qgcPal; colorGroupEnabled: enabled }

    function capitalizeWords(sentence) {
        return sentence.replace(/(?:^|\s)\S/g, a => a.toUpperCase())
    }

    component SectionCaption: QGCLabel {
        font.pointSize:     ScreenTools.smallFontPointSize
        font.letterSpacing: 0.5
        color:              qgcPal.colorGrey
        leftPadding:        _pad
    }

    QGCFlickable {
        clip:               true
        anchors.fill:       parent
        contentHeight:      summaryColumn.height + _fh
        contentWidth:       _summaryRoot.width
        flickableDirection: Flickable.VerticalFlick

        Column {
            id:             summaryColumn
            width:          _summaryRoot.width
            spacing:        _fh * 0.9

            Rectangle {
                id:     heroCard
                width:  parent.width
                height: heroRow.implicitHeight + _pad * 2
                radius: _radius
                color:  Qt.alpha(qgcPal.text, 0.055)

                RowLayout {
                    id:                 heroRow
                    anchors.fill:       parent
                    anchors.margins:    _pad
                    spacing:            _pad

                    Rectangle {
                        Layout.preferredWidth:  _fh * 4.2
                        Layout.preferredHeight: Layout.preferredWidth
                        radius:                 _fh * 0.8
                        color:                  Qt.alpha(qgcPal.colorBlue, 0.16)

                        QGCColoredImage {
                            anchors.centerIn:   parent
                            width:              parent.width * 0.62
                            height:             width
                            sourceSize.height:  height
                            fillMode:           Image.PreserveAspectFit
                            source:             "/InstrumentValueIcons/drone.svg"
                            color:              qgcPal.colorBlue
                        }
                    }

                    ColumnLayout {
                        Layout.fillWidth:   true
                        spacing:            _fh * 0.15

                        QGCLabel {
                            Layout.fillWidth:   true
                            text:               _vehicle ? _vehicle.vehicleTypeString : ""
                            font.pointSize:     ScreenTools.largeFontPointSize
                            font.bold:          true
                            elide:              Text.ElideRight
                        }

                        QGCLabel {
                            Layout.fillWidth:   true
                            text:               _firmwareText
                            color:              Qt.alpha(qgcPal.text, 0.6)
                            elide:              Text.ElideRight
                        }

                        QGCLabel {
                            Layout.fillWidth:   true
                            text:               _vehicle ? qsTr("Vehicle %1").arg(_vehicle.id) : ""
                            color:              Qt.alpha(qgcPal.text, 0.6)
                            font.pointSize:     ScreenTools.smallFontPointSize
                            elide:              Text.ElideRight
                        }
                    }

                    Rectangle {
                        objectName:             "setupStatusPill"
                        Layout.preferredWidth:  statusRow.implicitWidth + _fw * 2.4
                        Layout.preferredHeight: _fh * 1.8
                        Layout.alignment:       Qt.AlignVCenter
                        radius:                 height / 2
                        color:                  Qt.alpha(_statusColor, 0.16)

                        readonly property color _statusColor: _setupComplete ? qgcPal.colorGreen : qgcPal.colorOrange

                        Row {
                            id:                 statusRow
                            anchors.centerIn:   parent
                            spacing:            _fw * 0.8

                            Rectangle {
                                anchors.verticalCenter: parent.verticalCenter
                                width:                  _fh * 0.55
                                height:                 width
                                radius:                 width / 2
                                color:                  parent.parent._statusColor
                            }

                            QGCLabel {
                                anchors.verticalCenter: parent.verticalCenter
                                text:                   _setupComplete ? qsTr("Setup complete") : qsTr("Needs setup")
                                color:                  parent.parent._statusColor
                                font.bold:              true
                            }
                        }
                    }
                }
            }

            Column {
                width:      parent.width
                spacing:    _fh * 0.35
                visible:    !_setupComplete

                SectionCaption { text: qsTr("NEEDS SETUP") }

                PlanGroupCard {
                    width: parent.width

                    Repeater {
                        model: _components

                        PlanGroupRow {
                            text:           capitalizeWords(modelData.name)
                            description:    modelData.description
                            interactive:    true
                            showChevron:    true
                            visible:        modelData.requiresSetup && !modelData.setupComplete && modelData.setupSource.toString() !== ""
                            onClicked:      setupView.showVehicleComponentPanel(modelData)
                        }
                    }
                }
            }

            Column {
                width:      parent.width
                spacing:    _fh * 0.35

                SectionCaption { text: qsTr("COMPONENTS") }

                Flow {
                    width:      parent.width
                    spacing:    _cardSpacing

                    Repeater {
                        model: _components

                        Rectangle {
                            width:      _cardWidth
                            height:     cardColumn.height
                            radius:     _radius
                            color:      Qt.alpha(qgcPal.text, 0.055)
                            visible:    modelData.summaryQmlSource.toString() !== ""

                            readonly property bool _canOpen:    modelData.setupSource.toString() !== ""
                            readonly property bool _showStatus: modelData.requiresSetup && _canOpen
                            readonly property real _tileSize:   Math.round(_fh * 1.35)

                            Column {
                                id:     cardColumn
                                width:  parent.width

                                Item {
                                    width:  parent.width
                                    height: _fh * 2.6

                                    RowLayout {
                                        anchors.fill:           parent
                                        anchors.leftMargin:     _pad
                                        anchors.rightMargin:    _pad
                                        spacing:                _fw

                                        Rectangle {
                                            objectName:             "summaryTile" + modelData.name.replace(/\s/g, "")
                                            Layout.preferredWidth:  _tileSize
                                            Layout.preferredHeight: _tileSize
                                            radius:                 Math.round(_tileSize * 0.28)
                                            color:                  setupView.componentColor(modelData)

                                            QGCColoredImage {
                                                anchors.centerIn:   parent
                                                width:              Math.round(_tileSize * 0.68)
                                                height:             width
                                                sourceSize.height:  height
                                                fillMode:           Image.PreserveAspectFit
                                                source:             setupView.componentIcon(modelData)
                                                color:              "white"
                                            }
                                        }

                                        QGCLabel {
                                            Layout.fillWidth:   true
                                            text:               capitalizeWords(modelData.name)
                                            font.bold:          true
                                            elide:              Text.ElideRight
                                        }

                                        QGCColoredImage {
                                            Layout.preferredWidth:  _fh * 0.9
                                            Layout.preferredHeight: Layout.preferredWidth
                                            sourceSize.height:      height
                                            source:                 "/InstrumentValueIcons/checkmark-outline.svg"
                                            color:                  qgcPal.colorGreen
                                            visible:                _showStatus && modelData.setupComplete
                                        }

                                        Rectangle {
                                            Layout.preferredWidth:  _fh * 0.55
                                            Layout.preferredHeight: Layout.preferredWidth
                                            radius:                 width / 2
                                            color:                  qgcPal.colorRed
                                            visible:                _showStatus && !modelData.setupComplete
                                        }

                                        QGCLabel {
                                            text:           "›"
                                            color:          Qt.alpha(qgcPal.text, 0.4)
                                            font.pointSize: ScreenTools.mediumFontPointSize
                                            visible:        _canOpen
                                        }
                                    }

                                    QGCMouseArea {
                                        anchors.fill:   parent
                                        enabled:        _canOpen
                                        onClicked:      setupView.showVehicleComponentPanel(modelData)
                                    }
                                }

                                Rectangle {
                                    x:      _pad
                                    width:  parent.width - _pad
                                    height: 1
                                    color:  Qt.alpha(qgcPal.text, 0.09)
                                }

                                Item {
                                    width:  parent.width
                                    height: summaryLoader.height + _fh * 0.6

                                    Loader {
                                        id:                 summaryLoader
                                        x:                  _pad
                                        y:                  _fh * 0.3
                                        width:              parent.width - _pad * 2
                                        height:             item ? Math.max(0, ...Array.from(item.children).map(child => child.implicitHeight)) : 0
                                        source:             modelData.summaryQmlSource

                                        property var vehicleComponent: modelData
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
