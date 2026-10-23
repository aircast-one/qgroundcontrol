import QtQuick
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls

Column {
    id:         _root
    spacing:    ScreenTools.defaultFontPixelHeight / 2

    property var    _recorder:      globals.routeRecorder
    property string _savedFile:     ""
    property string _message:       ""
    property bool   _showNotice:    false
    property real   _margins:       ScreenTools.defaultFontPixelWidth

    QGCPalette { id: qgcPal }

    Connections {
        target: _recorder

        function onFinished(fileName, message) {
            _savedFile = fileName
            _message = message
            _showNotice = true
            noticeTimer.restart()
        }

        function onRecordingChanged() {
            if (_recorder.recording) {
                _showNotice = false
            }
        }
    }

    Timer {
        id:         noticeTimer
        interval:   15000
        onTriggered: _showNotice = false
    }

    Rectangle {
        anchors.horizontalCenter:   parent.horizontalCenter
        width:                      recordingRow.width + (_margins * 2)
        height:                     recordingRow.height + _margins
        radius:                     height / 2
        color:                      Qt.rgba(qgcPal.window.r, qgcPal.window.g, qgcPal.window.b, 0.85)
        visible:                    _recorder.recording

        Row {
            id:                 recordingRow
            anchors.centerIn:   parent
            spacing:            _margins

            Rectangle {
                anchors.verticalCenter: parent.verticalCenter
                width:                  ScreenTools.defaultFontPixelHeight * 0.7
                height:                 width
                radius:                 width / 2
                color:                  "red"

                SequentialAnimation on opacity {
                    running:    _recorder.recording
                    loops:      Animation.Infinite
                    NumberAnimation { from: 1; to: 0.2; duration: 600 }
                    NumberAnimation { from: 0.2; to: 1; duration: 600 }
                }
            }

            QGCLabel {
                text: (globals.activeVehicle && globals.activeVehicle.flying) || _recorder.distance > 0
                      ? qsTr("Recording route · %1").arg(QGroundControl.unitsConversion.metersToAppSettingsHorizontalDistanceUnitsString(_recorder.distance))
                      : qsTr("Recording route · starts when the vehicle is flying")
            }
        }
    }

    Rectangle {
        anchors.horizontalCenter:   parent.horizontalCenter
        width:                      noticeRow.width + (_margins * 2)
        height:                     noticeRow.height + _margins
        radius:                     _margins
        color:                      Qt.rgba(qgcPal.window.r, qgcPal.window.g, qgcPal.window.b, 0.9)
        visible:                    _showNotice

        RowLayout {
            id:                 noticeRow
            anchors.centerIn:   parent
            spacing:            _margins

            QGCLabel { text: _message }

            QGCButton {
                text:       qsTr("Open in Plan")
                visible:    _savedFile !== ""
                onClicked: {
                    _showNotice = false
                    mainWindow.openPlanFile(_savedFile)
                }
            }

            QGCButton {
                text:       qsTr("Dismiss")
                onClicked:  _showNotice = false
            }
        }
    }
}
