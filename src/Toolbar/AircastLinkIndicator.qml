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
            size:                   parent.height * 0.8
            percent:                _qualityKnown ? _quality : 0
        }

        Column {
            anchors.verticalCenter: parent.verticalCenter

            QGCLabel {
                text:           _qualityKnown ? _quality + "%" : "--"
                font.pointSize: ScreenTools.smallFontPointSize
            }

            QGCLabel {
                text:           bitrateText(_bitrateKbps)
                font.pointSize: ScreenTools.smallFontPointSize
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
                heading: qsTr("Cellular link")

                LabelledLabel {
                    label:      qsTr("Signal:")
                    labelText:  _qualityKnown ? _quality + " %" : qsTr("unknown")
                }

                LabelledLabel {
                    label:      qsTr("Network:")
                    labelText:  _link.radioType.enumStringValue
                }

                LabelledLabel {
                    label:      qsTr("Modem:")
                    labelText:  _link.status.enumStringValue
                }

                LabelledLabel {
                    label:      qsTr("Video bitrate:")
                    labelText:  bitrateText(_bitrateKbps)
                }

                QGCLabel {
                    text:           qsTr("Signal, up to the last hour")
                    font.pointSize: ScreenTools.smallFontPointSize
                }

                Sparkline {
                    values:     _link.qualityHistory
                    maximum:    100
                }

                QGCLabel {
                    text:           qsTr("Video bitrate, up to the last hour")
                    font.pointSize: ScreenTools.smallFontPointSize
                }

                Sparkline {
                    values:     _link.bitrateHistory
                    maximum:    0
                }
            }
        }
    }
}
