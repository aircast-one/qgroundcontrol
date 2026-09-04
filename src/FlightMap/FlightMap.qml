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
import QtQuick.Shapes
import QtLocation
import QtPositioning
import QtQuick.Dialogs
import Qt.labs.animation

import QGroundControl
import QGroundControl.FactSystem
import QGroundControl.Controls
import QGroundControl.FlightMap
import QGroundControl.ScreenTools
import QGroundControl.MultiVehicleManager
import QGroundControl.Vehicle
import QGroundControl.QGCPositionManager

Map {
    id: _map

    plugin:     Plugin { name: "QGroundControl" }
    opacity:    0.99

    property string mapName:                        'defaultMap'
    property bool   isSatelliteMap:                 activeMapType.name.indexOf("Satellite") > -1 || activeMapType.name.indexOf("Hybrid") > -1
    property var    gcsPosition:                    QGroundControl.qgcPositionManger.gcsPosition
    property real   gcsHeading:                     QGroundControl.qgcPositionManger.gcsHeading
    property bool   allowGCSLocationCenter:         false
    property bool   allowVehicleLocationCenter:     false
    property bool   firstGCSPositionReceived:       false
    property bool   firstVehiclePositionReceived:   false
    property bool   planView:                       false

    property var    _activeVehicle:             QGroundControl.multiVehicleManager.activeVehicle
    property var    _activeVehicleCoordinate:   _activeVehicle ? _activeVehicle.coordinate : QtPositioning.coordinate()

    function setVisibleRegion(region) {
        let maxZoomLevel = 20
        _map.visibleRegion = QtPositioning.rectangle(QtPositioning.coordinate(0, 0), QtPositioning.coordinate(0, 0))
        _map.visibleRegion = region
        if (_map.zoomLevel > maxZoomLevel) {
            _map.zoomLevel = maxZoomLevel
        }
    }

    function _possiblyCenterToVehiclePosition() {
        if (!firstVehiclePositionReceived && allowVehicleLocationCenter && _activeVehicleCoordinate.isValid) {
            firstVehiclePositionReceived = true
            center = _activeVehicleCoordinate
            zoomLevel = QGroundControl.flightMapInitialZoom
        }
    }

    function centerToSpecifiedLocation() {
        specifyMapPositionDialog.createObject(mainWindow).open()
    }

    Component {
        id: specifyMapPositionDialog
        EditPositionDialog {
            title:                  qsTr("Specify Position")
            coordinate:             center
            onCoordinateChanged:    center = coordinate
        }
    }

    onGcsPositionChanged: {
        if (gcsPosition.isValid && allowGCSLocationCenter && !firstGCSPositionReceived && !firstVehiclePositionReceived) {
            firstGCSPositionReceived = true
            var _activeVehicleCoordinate = _activeVehicle ? _activeVehicle.coordinate : QtPositioning.coordinate()
            if(QGroundControl.settingsManager.flyViewSettings.keepMapCenteredOnVehicle.rawValue || !_activeVehicleCoordinate.isValid)
                center = gcsPosition
        }
    }

    function updateActiveMapType() {
        var settings =  QGroundControl.settingsManager.flightMapSettings
        var fullMapName = settings.mapProvider.value + " " + settings.mapType.value

        for (var i = 0; i < _map.supportedMapTypes.length; i++) {
            if (fullMapName === _map.supportedMapTypes[i].name) {
                _map.activeMapType = _map.supportedMapTypes[i]
                return
            }
        }
    }

    on_ActiveVehicleCoordinateChanged: _possiblyCenterToVehiclePosition()

    onMapReadyChanged: {
        if (_map.mapReady) {
            updateActiveMapType()
            _possiblyCenterToVehiclePosition()
        }
    }

    Connections {
        target: QGroundControl.settingsManager.flightMapSettings.mapType
        function onRawValueChanged() { updateActiveMapType() }
    }

    Connections {
        target: QGroundControl.settingsManager.flightMapSettings.mapProvider
        function onRawValueChanged() { updateActiveMapType() }
    }

    signal mapPanStart
    signal mapPanStop
    signal mapClicked(var position)
    
    function _zoomAbout(point, levels) {
        const anchor = _map.toCoordinate(point, false)
        _map.zoomLevel += levels
        _map.alignCoordinateToPoint(anchor, point)
    }

    PinchHandler {
        id:     pinchHandler
        target: null

        onScaleChanged: (delta) => _zoomAbout(pinchHandler.centroid.position, Math.log2(delta))
    }

    property bool _wheelPanning: false

    WheelHandler {
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad

        onWheel: (event) => {
            const zoomModifier = event.modifiers & (Qt.ControlModifier | Qt.MetaModifier)
            if (event.device.type !== PointerDevice.TouchPad || zoomModifier) {
                _zoomAbout(Qt.point(event.x, event.y), event.angleDelta.y / 120)
                return
            }
            if (event.buttons) return
            if (event.phase === Qt.ScrollBegin) {
                _wheelPanning = true
                mapPanStart()
            }
            const delta = event.pixelDelta.x || event.pixelDelta.y ? event.pixelDelta
                                                                  : Qt.point(event.angleDelta.x / 2, event.angleDelta.y / 2)
            _map.pan(-delta.x, -delta.y)
            if (event.phase === Qt.ScrollEnd && _wheelPanning) {
                _wheelPanning = false
                mapPanStop()
            }
        }
    }

    property bool panEnabled: true

    MultiPointTouchArea {
        anchors.fill: parent
        maximumTouchPoints: 1
        mouseEnabled: true
        enabled: _map.panEnabled

        property bool dragActive: false
        property real lastMouseX
        property real lastMouseY

        onCanceled: {
            if (dragActive) {
                dragActive = false
                mapPanStop()
            }
        }

        onPressed: (touchPoints) => {
            lastMouseX = touchPoints[0].x
            lastMouseY = touchPoints[0].y
        }

        onGestureStarted: (gesture) => {
            dragActive = true
            gesture.grab()
            mapPanStart()
        }

        onUpdated: (touchPoints) => {
            if (dragActive) {
                let deltaX = touchPoints[0].x - lastMouseX
                let deltaY = touchPoints[0].y - lastMouseY
                if (Math.abs(deltaX) >= 1.0 || Math.abs(deltaY) >= 1.0) {
                    _map.pan(lastMouseX - touchPoints[0].x, lastMouseY - touchPoints[0].y)
                    lastMouseX = touchPoints[0].x
                    lastMouseY = touchPoints[0].y
                }
            }
        }

        onReleased: (touchPoints) => {
            if (dragActive) {
                _map.pan(lastMouseX - touchPoints[0].x, lastMouseY - touchPoints[0].y)
                dragActive = false
                mapPanStop()
            } else {
                mapClicked(Qt.point(touchPoints[0].x, touchPoints[0].y))
            }
        }
    }

    MapQuickItem {
        anchorPoint.x:  sourceItem.width / 2
        anchorPoint.y:  sourceItem.height / 2
        visible:        gcsPosition.isValid
        coordinate:     gcsPosition

        sourceItem: Item {
            id:     gcsMarker
            width:  ScreenTools.defaultFontPixelHeight * 2.6
            height: width

            readonly property color _locationBlue: "#2f7cf6"

            Rectangle {
                anchors.fill:   parent
                radius:         width / 2
                color:          Qt.alpha(parent._locationBlue, 0.2)
            }

            Shape {
                anchors.fill:   parent
                visible:        !isNaN(gcsHeading)
                rotation:       isNaN(gcsHeading) ? 0 : gcsHeading
                antialiasing:   true

                ShapePath {
                    strokeWidth:    0
                    strokeColor:    "transparent"
                    fillColor:      Qt.alpha(gcsMarker._locationBlue, 0.85)
                    startX:         gcsMarker.width / 2
                    startY:         0

                    PathLine { x: gcsMarker.width * 0.72; y: gcsMarker.height * 0.36 }
                    PathLine { x: gcsMarker.width * 0.28; y: gcsMarker.height * 0.36 }
                    PathLine { x: gcsMarker.width / 2;    y: 0 }
                }
            }

            Rectangle {
                anchors.centerIn:   parent
                width:              parent.width * 0.52
                height:             width
                radius:             width / 2
                color:              "white"
            }

            Rectangle {
                anchors.centerIn:   parent
                width:              parent.width * 0.38
                height:             width
                radius:             width / 2
                color:              parent._locationBlue
            }
        }
    }
}
