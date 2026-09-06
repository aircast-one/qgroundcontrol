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
import QtLocation
import QtPositioning
import QtQuick.Dialogs

import QGroundControl
import QGroundControl.ScreenTools
import QGroundControl.Palette
import QGroundControl.Controls
import QGroundControl.FlightMap
import QGroundControl.ShapeFileHelper

Item {
    id: _root

    property var    mapControl
    property var    mapPolyline
    property bool   interactive:    mapPolyline.interactive
    property bool   defaultShapeWhenEmpty: false
    property int    lineWidth:      3
    property color  lineColor:      "#be781c"

    property var    _dragHandlesComponent
    property var    _splitHandlesComponent
    property real   _zorderDragHandle:      QGroundControl.zOrderMapItems + 3
    property real   _zorderSplitHandle:     QGroundControl.zOrderMapItems + 2

    readonly property var _units: QGroundControl.unitsConversion

    function _length() {
        const path = mapPolyline.path
        return path.slice(1).reduce((sum, vertex, i) => sum + path[i].distanceTo(vertex), 0)
    }

    function _shapeCaption() {
        if (mapPolyline.count < 2) {
            return qsTr("Polyline Tools")
        }
        return qsTr("Length %1").arg(_units.formatMeasure(_units.metersToAppSettingsHorizontalDistanceUnits(_length()), _units.appSettingsHorizontalDistanceUnitsString))
    }

    function _traceCaption() {
        return mapPolyline.traceComplete ? qsTr("%1 points").arg(mapPolyline.count)
                                         : qsTr("Click the map to add points \u00B7 %1 of %2").arg(mapPolyline.count).arg(mapPolyline.minVertexCount)
    }

    function _addCommonVisuals() {
        if (_objMgrCommonVisuals.empty) {
            _objMgrCommonVisuals.createObject(polylineComponent, mapControl, true)
        }
    }

    function _addInteractiveVisuals() {
        if (_objMgrInteractiveVisuals.empty) {
            _objMgrInteractiveVisuals.createObjects([ dragHandlesComponent, splitHandlesComponent ], mapControl)
            _objMgrInteractiveVisuals.createObject(toolbarComponent, OverlayBackdrop.chromeLayer || mapControl, false)
        }
    }

    function _defaultPolylineVertices() {
        var x = mapControl.centerViewport.x + (mapControl.centerViewport.width / 2)
        var yInset = mapControl.centerViewport.height / 4
        var topPointCoord =     mapControl.toCoordinate(Qt.point(x, mapControl.centerViewport.y + yInset),                                false /* clipToViewPort */)
        var bottomPointCoord =  mapControl.toCoordinate(Qt.point(x, mapControl.centerViewport.y + mapControl.centerViewport.height - yInset),    false /* clipToViewPort */)
        return [ topPointCoord, bottomPointCoord ]
    }

    function _resetPolyline() {
        mapPolyline.beginReset()
        mapPolyline.clear()
        var initialVertices = _defaultPolylineVertices()
        mapPolyline.appendVertex(initialVertices[0])
        mapPolyline.appendVertex(initialVertices[1])
        mapPolyline.endReset()
    }

    function _handleInteractiveChanged() {
        if (interactive) {
            if (defaultShapeWhenEmpty && mapPolyline.count === 0) {
                _resetPolyline()
            }
            _addInteractiveVisuals()
        } else {
            _objMgrInteractiveVisuals.destroyObjects()
        }
    }

    onInteractiveChanged: _handleInteractiveChanged()

    Connections {
        target: mapPolyline
        onTraceModeChanged: {
            if (mapPolyline.traceMode) {
                _objMgrTraceVisuals.createObject(traceMouseAreaComponent, mapControl, false)
            } else {
                _objMgrTraceVisuals.destroyObjects()
            }
        }
    }

    Component.onCompleted: {
        _addCommonVisuals()
        if (interactive) {
            _handleInteractiveChanged()
        }
    }
    Component.onDestruction: mapPolyline.traceMode = false

    QGCDynamicObjectManager { id: _objMgrCommonVisuals }
    QGCDynamicObjectManager { id: _objMgrInteractiveVisuals }
    QGCDynamicObjectManager { id: _objMgrTraceVisuals }

    QGCPalette { id: qgcPal }

    KMLOrSHPFileDialog {
        id:             kmlOrSHPLoadDialog
        title:          qsTr("Select Polyline File")

        onAcceptedForLoad: (file) => {
            mapPolyline.loadKMLOrSHPFile(file)
            mapFitFunctions.fitMapViewportToMissionItems()
            close()
        }
    }

    QGCMenu {
        id: menu
        property int _removeVertexIndex

        function popUpWithIndex(curIndex) {
            _removeVertexIndex = curIndex
            removeVertexItem.visible = mapPolyline.count > 2
            menu.popup()
        }

        QGCMenuItem {
            id:             removeVertexItem
            text:           qsTr("Remove vertex" )
            onTriggered:    mapPolyline.removeVertex(menu._removeVertexIndex)
        }

        QGCMenuItem {
            text:           qsTr("Edit position..." )
            onTriggered:    editPositionDialog.createObject(mainWindow, { coordinate: mapPolyline.path[menu._removeVertexIndex] }).open()
        }
    }

    Component {
        id: editPositionDialog

        EditPositionDialog {
            onCoordinateChanged: mapPolyline.adjustVertex(menu._removeVertexIndex,coordinate)
        }
    }

    Component {
        id: polylineComponent

        MapPolyline {
            line.width: lineWidth
            line.color: lineColor
            path:       mapPolyline.path
            visible:    _root.visible
            opacity:    _root.opacity
        }
    }

    Component {
        id: splitHandleComponent

        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  sourceItem.width / 2
            anchorPoint.y:  sourceItem.height / 2
            z:              _zorderSplitHandle
            opacity:        _root.opacity

            property int vertexIndex

            sourceItem: SplitIndicator {
                onClicked:  mapPolyline.splitSegment(mapQuickItem.vertexIndex)
            }
        }
    }

    Component {
        id: splitHandlesComponent

        Repeater {
            model: mapPolyline.path

            delegate: Item {
                property var _splitHandle
                property var _vertices:     mapPolyline.path

                opacity:    _root.opacity

                function _setHandlePosition() {
                    var nextIndex = index + 1
                    var distance = _vertices[index].distanceTo(_vertices[nextIndex])
                    var azimuth = _vertices[index].azimuthTo(_vertices[nextIndex])
                    _splitHandle.coordinate = _vertices[index].atDistanceAndAzimuth(distance / 2, azimuth)
                }

                Component.onCompleted: {
                    if (index + 1 <= _vertices.length - 1) {
                        _splitHandle = splitHandleComponent.createObject(mapControl)
                        _splitHandle.vertexIndex = index
                        _setHandlePosition()
                        mapControl.addMapItem(_splitHandle)
                    }
                }

                Component.onDestruction: {
                    if (_splitHandle) {
                        _splitHandle.destroy()
                    }
                }
            }
        }
    }

    Component {
        id: dragAreaComponent

        MissionItemIndicatorDrag {
            mapControl: _root.mapControl
            id:         dragArea
            z:          _zorderDragHandle
            opacity:    _root.opacity

            property int polylineVertex

            property bool _creationComplete: false

            Component.onCompleted: _creationComplete = true

            onItemCoordinateChanged: {
                if (_creationComplete) {
                    mapPolyline.adjustVertex(polylineVertex, itemCoordinate)
                }
            }

            onClicked: {
                menu.popUpWithIndex(polylineVertex)
            }

        }
    }

    Component {
        id: dragHandleComponent

        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  dragHandle.width / 2
            anchorPoint.y:  dragHandle.height / 2
            z:              _zorderDragHandle
            opacity:        _root.opacity

            property int polylineVertex

            sourceItem: MapEditHandle { id: dragHandle }
        }
    }

    Component {
        id: dragHandlesComponent

        Repeater {
            model: mapPolyline.pathModel

            delegate: Item {
                property var _visuals: [ ]

                opacity:    _root.opacity

                Component.onCompleted: {
                    var dragHandle = dragHandleComponent.createObject(mapControl)
                    dragHandle.coordinate = Qt.binding(function() { return object.coordinate })
                    dragHandle.polylineVertex = Qt.binding(function() { return index })
                    mapControl.addMapItem(dragHandle)
                    var dragArea = dragAreaComponent.createObject(mapControl, { "itemIndicator": dragHandle, "itemCoordinate": object.coordinate })
                    dragArea.polylineVertex = Qt.binding(function() { return index })
                    _visuals.push(dragHandle)
                    _visuals.push(dragArea)
                }

                Component.onDestruction: {
                    for (var i=0; i<_visuals.length; i++) {
                        _visuals[i].destroy()
                    }
                    _visuals = [ ]
                }
            }
        }
    }

    Component {
        id: toolbarComponent

        PlanEditToolbar {
            mapControl:                     _root.mapControl
            z:                              QGroundControl.zOrderMapItems + 2
            caption:                        mapPolyline.traceMode ? _traceCaption() : _shapeCaption()
            accentEnabled:                  mapPolyline.traceComplete
            tools: mapPolyline.traceMode
                ? [ { text: qsTr("Cancel"), action: mapPolyline.cancelTrace },
                    { text: qsTr("Done"), accent: true, action: mapPolyline.finishTrace } ]
                : [ { text: qsTr("Line"),    action: _resetPolyline },
                    { separator: true },
                    { text: qsTr("Trace"),   action: mapPolyline.beginTrace },
                    { text: qsTr("Import…"), action: kmlOrSHPLoadDialog.openForLoad } ]
        }
    }

    Component {
        id:  traceMouseAreaComponent

        MouseArea {
            anchors.fill:       mapControl
            preventStealing:    true
            z:                  QGroundControl.zOrderMapItems + 1

            onClicked: (mouse) => {
                if (mouse.button === Qt.LeftButton && _root.interactive) {
                    mapPolyline.appendVertex(mapControl.toCoordinate(Qt.point(mouse.x, mouse.y), false /* clipToViewPort */))
                }
            }
        }
    }
}

