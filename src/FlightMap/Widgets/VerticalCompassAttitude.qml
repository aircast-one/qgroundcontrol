import QtQuick

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlightMap

Rectangle {
    width:  Math.min(_defaultWidth, _maxWidth)
    height: _outerRadius * 4
    radius: _outerRadius
    color:  "transparent"

    OverlayGlass {
        anchors.fill: parent
        radius:       parent.radius
    }

    property real extraInset:           0
    property real extraValuesWidth:     _outerRadius

    property real _defaultWidth: mainWindow.width * 0.22
    property real _maxWidth:     ScreenTools.defaultFontPixelHeight * 10

    property real _outerMargin: (width * 0.05) / 2
    property real _outerRadius: width / 2
    property real _innerRadius: _outerRadius - _outerMargin

    // Prevent all clicks from going through to lower layers
    DeadMouseArea {
        anchors.fill: parent
    }

    QGCAttitudeWidget {
        id:                         attitude
        anchors.horizontalCenter:   parent.horizontalCenter
        anchors.topMargin:          _outerMargin
        anchors.top:                parent.top
        size:                       _innerRadius * 2
        vehicle:                    globals.activeVehicle
    }

    QGCCompassWidget {
        id:                         compass
        anchors.horizontalCenter:   parent.horizontalCenter
        anchors.topMargin:          _outerMargin * 2
        anchors.top:                attitude.bottom
        size:                       _innerRadius * 2
        vehicle:                    globals.activeVehicle
    }
}
