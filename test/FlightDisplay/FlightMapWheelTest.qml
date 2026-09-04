import QtQuick
import QtPositioning

import QGroundControl.FlightMap

FlightMap {
    width:      600
    height:     400
    center:     QtPositioning.coordinate(47.4, 8.5)
    zoomLevel:  10

    property int panStarts: 0
    property int panStops:  0

    onMapPanStart:  panStarts++
    onMapPanStop:   panStops++
}
