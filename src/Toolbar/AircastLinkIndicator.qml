import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

Item {
    id:             control
    objectName:     "toolbar_aircastLinkIndicator"
    anchors.top:    parent.top
    anchors.bottom: parent.bottom
    width:          row.width

    property bool showIndicator: _link.telemetryAvailable

    property var    _activeVehicle: QGroundControl.multiVehicleManager.activeVehicle
    property var    _link:          _activeVehicle.aircastLink
    property int    _quality:       _link.quality.rawValue
    property bool   _qualityKnown:  _quality !== 255
    property int    _bitrateKbps:   _link.videoBitrate.rawValue

    function bitrateText(kbps) {
        return kbps >= 1000 ? (kbps / 1000).toFixed(1) + " " + qsTr("Mbit/s") : kbps + " " + qsTr("kbit/s")
    }

    Row {
        id:             row
        anchors.top:    parent.top
        anchors.bottom: parent.bottom
        spacing:        ScreenTools.defaultFontPixelWidth / 2

        SignalStrength {
            anchors.verticalCenter: parent.verticalCenter
            visible:                _link.cellular
            size:                   parent.height * 0.8
            percent:                _qualityKnown ? _quality : 0
        }

        Column {
            anchors.verticalCenter: parent.verticalCenter

            QGCLabel {
                text:           _link.linkText
                font.pointSize: ScreenTools.smallFontPointSize
            }

            QGCLabel {
                text:           bitrateText(_bitrateKbps)
                font.pointSize: ScreenTools.smallFontPointSize
                visible:        _link.cellular
            }
        }
    }

    MouseArea {
        anchors.fill:   parent
        onClicked:      mainWindow.showIndicatorDrawer(linkInfoPage, control)
    }

    Component {
        id: linkInfoPage

        ToolIndicatorPage {
            showExpand: false

            contentComponent: SettingsGroupLayout {
                heading: qsTr("Aircraft link")

                LabelledLabel {
                    label:      qsTr("Uplink:")
                    labelText:  _link.uplink.enumStringValue
                    visible:    _link.uplinkKnown
                }

                LabelledLabel {
                    label:      qsTr("Signal:")
                    labelText:  _qualityKnown ? _quality + " %" : qsTr("unknown")
                    visible:    _link.cellular
                }

                LabelledLabel {
                    label:      qsTr("Network:")
                    labelText:  _link.radioType.enumStringValue
                    visible:    _link.cellular
                }

                LabelledLabel {
                    label:      qsTr("Modem:")
                    labelText:  _link.status.enumStringValue
                    visible:    _link.cellular
                }

                LabelledLabel {
                    label:      qsTr("Video bitrate:")
                    labelText:  bitrateText(_bitrateKbps)
                    visible:    _link.cellular
                }

                QGCLabel {
                    text:           qsTr("Signal, up to the last hour")
                    font.pointSize: ScreenTools.smallFontPointSize
                    visible:        _link.cellular
                }

                Sparkline {
                    values:     _link.qualityHistory
                    maximum:    100
                    visible:    _link.cellular
                }

                QGCLabel {
                    text:           qsTr("Video bitrate, up to the last hour")
                    font.pointSize: ScreenTools.smallFontPointSize
                    visible:        _link.cellular
                }

                Sparkline {
                    values:     _link.bitrateHistory
                    maximum:    0
                    visible:    _link.cellular
                }

                ColumnLayout {
                    spacing: 0
                    visible: _link.uplinkSwitches.length > 0

                    QGCLabel {
                        text:           qsTr("Switches")
                        font.pointSize: ScreenTools.smallFontPointSize
                    }

                    Repeater {
                        model: _link.uplinkSwitches

                        QGCLabel {
                            text: modelData.clock + "  " + modelData.from + " → " + modelData.to
                        }
                    }
                }
            }
        }
    }
}
