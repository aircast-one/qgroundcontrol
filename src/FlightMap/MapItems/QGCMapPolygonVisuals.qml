import QtQuick
import QtQuick.Controls
import QtLocation
import QtPositioning
import QtQuick.Dialogs
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlightMap
import QGroundControl.PlanView

Item {
    id: _root

    property var    mapControl
    property var    mapPolygon
    property bool   interactive:        mapPolygon.interactive
    property bool   defaultShapeWhenEmpty: false
    property color  interiorColor:      "transparent"
    property color  altColor:           "transparent"
    property real   interiorOpacity:    1
    property int    borderWidth:        0
    property color  borderColor:        "black"

    property bool   _circleMode:                false
    property real   _circleRadius
    property bool   _circleRadiusDrag:          false
    property var    _circleRadiusDragCoord:     QtPositioning.coordinate()
    property bool   _editCircleRadius:          false
    property string _instructionText:           _polygonToolsText
    property var    _savedVertices:             [ ]
    property bool   _savedCircleMode
    property bool   _isVertexBeingDragged:      false

    property real _zorderDragHandle:    QGroundControl.zOrderMapItems + 3
    property real _zorderSplitHandle:   QGroundControl.zOrderMapItems + 2
    property real _zorderCenterHandle:  QGroundControl.zOrderMapItems + 1

    readonly property var _units: QGroundControl.unitsConversion

    function _perimeter() {
        const path = mapPolygon.path
        return path.reduce((sum, vertex, i) => sum + vertex.distanceTo(path[(i + 1) % path.length]), 0)
    }

    function _distanceText(meters) {
        return _units.formatMeasure(_units.metersToAppSettingsHorizontalDistanceUnits(meters), _units.appSettingsHorizontalDistanceUnitsString)
    }

    function _shapeCaption() {
        if (mapPolygon.count === 0) {
            return qsTr("Polygon Tools")
        }
        if (_circleMode) {
            return qsTr("Radius %1").arg(_distanceText(_circleRadius))
        }
        return _units.formatMeasure(_units.squareMetersToAppSettingsAreaUnits(mapPolygon.area), _units.appSettingsAreaUnitsString) + " \u00B7 " + _distanceText(_perimeter())
    }

    function _traceCaption() {
        return mapPolygon.traceComplete ? qsTr("%1 points").arg(mapPolygon.count)
                                        : qsTr("Click the map to add points \u00B7 %1 of %2").arg(mapPolygon.count).arg(mapPolygon.minVertexCount)
    }

    function addCommonVisuals() {
        if (_objMgrCommonVisuals.empty) {
            _objMgrCommonVisuals.createObject(polygonComponent, mapControl, true)
        }
    }

    function removeCommonVisuals() {
        _objMgrCommonVisuals.destroyObjects()
    }

    function addEditingVisuals() {
        if (_objMgrEditingVisuals.empty) {
            _objMgrEditingVisuals.createObjects(
                [ dragHandlesComponent, splitHandlesComponent, centerDragHandleComponent, edgeLengthHandlesComponent ],
                mapControl,
                false /* addToMap */)
        }
    }

    function removeEditingVisuals() {
        _objMgrEditingVisuals.destroyObjects()
    }

    function addToolbarVisuals() {
        if (_objMgrToolVisuals.empty) {
            var toolbar = _objMgrToolVisuals.createObject(toolbarComponent, OverlayBackdrop.chromeLayer || mapControl)
            toolbar.z = QGroundControl.zOrderWidgets
        }
    }

    function removeToolVisuals() {
        _objMgrToolVisuals.destroyObjects()
    }

    function addCircleVisuals() {
        if (_objMgrCircleVisuals.empty) {
            _objMgrCircleVisuals.createObject(radiusVisualsComponent, mapControl)
        }
    }

    function defaultPolygonVertices() {
        var rect = Qt.rect(mapControl.centerViewport.x, mapControl.centerViewport.y, mapControl.centerViewport.width, mapControl.centerViewport.height)
        rect.x += (rect.width * 0.25) / 2
        rect.y += (rect.height * 0.25) / 2
        rect.width *= 0.75
        rect.height *= 0.75

        var centerCoord =       mapControl.toCoordinate(Qt.point(rect.x + (rect.width / 2), rect.y + (rect.height / 2)),   false /* clipToViewPort */)
        var topLeftCoord =      mapControl.toCoordinate(Qt.point(rect.x, rect.y),                                          false /* clipToViewPort */)
        var topRightCoord =     mapControl.toCoordinate(Qt.point(rect.x + rect.width, rect.y),                             false /* clipToViewPort */)
        var bottomLeftCoord =   mapControl.toCoordinate(Qt.point(rect.x, rect.y + rect.height),                            false /* clipToViewPort */)
        var bottomRightCoord =  mapControl.toCoordinate(Qt.point(rect.x + rect.width, rect.y + rect.height),               false /* clipToViewPort */)

        var halfWidthMeters =   Math.min(topLeftCoord.distanceTo(topRightCoord), 3000) / 2
        var halfHeightMeters =  Math.min(topLeftCoord.distanceTo(bottomLeftCoord), 3000) / 2
        topLeftCoord =      centerCoord.atDistanceAndAzimuth(halfWidthMeters, -90).atDistanceAndAzimuth(halfHeightMeters, 0)
        topRightCoord =     centerCoord.atDistanceAndAzimuth(halfWidthMeters, 90).atDistanceAndAzimuth(halfHeightMeters, 0)
        bottomLeftCoord =   centerCoord.atDistanceAndAzimuth(halfWidthMeters, -90).atDistanceAndAzimuth(halfHeightMeters, 180)
        bottomRightCoord =  centerCoord.atDistanceAndAzimuth(halfWidthMeters, 90).atDistanceAndAzimuth(halfHeightMeters, 180)

        return [ topLeftCoord, topRightCoord, bottomRightCoord, bottomLeftCoord  ]
    }

    function _resetPolygon() {
        mapPolygon.beginReset()
        mapPolygon.clear()
        mapPolygon.appendVertices(defaultPolygonVertices())
        mapPolygon.endReset()
        _circleMode = false
    }

    function _createCircularPolygon(center, radius) {
        var unboundCenter = center.atDistanceAndAzimuth(0, 0)
        var segments = 16
        var angleIncrement = 360 / segments
        var angle = 0
        mapPolygon.beginReset()
        mapPolygon.clear()
        _circleRadius = radius
        for (var i=0; i<segments; i++) {
            var vertex = unboundCenter.atDistanceAndAzimuth(radius, angle)
            mapPolygon.appendVertex(vertex)
            angle += angleIncrement
        }
        mapPolygon.endReset()
        _circleMode = true
    }

    function _resetCircle() {
        var initialVertices = defaultPolygonVertices()
        var width = initialVertices[0].distanceTo(initialVertices[1])
        var height = initialVertices[1].distanceTo(initialVertices[2])
        var radius = Math.min(width, height) / 2
        var center = initialVertices[0].atDistanceAndAzimuth(width / 2, 90).atDistanceAndAzimuth(height / 2, 180)
        _createCircularPolygon(center, radius)
    }

    function _handleInteractiveChanged() {
        if (interactive) {
            if (defaultShapeWhenEmpty && mapPolygon.count === 0) {
                _resetPolygon()
            }
            addEditingVisuals()
            addToolbarVisuals()
        } else {
            mapPolygon.traceMode = false
            removeEditingVisuals()
            removeToolVisuals()
        }
    }

    onInteractiveChanged: _handleInteractiveChanged()

    on_CircleModeChanged: {
        if (_circleMode) {
            addCircleVisuals()
        } else {
            _objMgrCircleVisuals.destroyObjects()
        }
    }

    Connections {
        target: mapPolygon
        function onTraceModeChanged(traceMode) {
            if (traceMode) {
                _instructionText = _traceText
                _objMgrTraceVisuals.createObject(traceMouseAreaComponent, mapControl, false)
            } else {
                _objMgrTraceVisuals.destroyObjects()
            }
        }
    }

    Component.onCompleted: {
        addCommonVisuals()
        _handleInteractiveChanged()
    }
    Component.onDestruction: mapPolygon.traceMode = false

    function _startTrace() {
        _savedCircleMode = _circleMode
        _circleMode = false
        mapPolygon.beginTrace()
    }

    function _cancelTrace() {
        mapPolygon.cancelTrace()
        _circleMode = _savedCircleMode
    }

    QGCDynamicObjectManager { id: _objMgrCommonVisuals }
    QGCDynamicObjectManager { id: _objMgrToolVisuals }
    QGCDynamicObjectManager { id: _objMgrEditingVisuals }
    QGCDynamicObjectManager { id: _objMgrTraceVisuals }
    QGCDynamicObjectManager { id: _objMgrCircleVisuals }

    QGCPalette { id: qgcPal }

    KMLOrSHPFileDialog {
        id:             kmlOrSHPLoadDialog
        title:          qsTr("Select Polygon File")

        onAcceptedForLoad: (file) => {
            mapPolygon.loadKMLOrSHPFile(file)
            mapFitFunctions.fitMapViewportToMissionItems()
            close()
        }
    }

    QGCMenu {
        id: menu

        property int _editingVertexIndex: -1

        function popupVertex(curIndex) {
            menu._editingVertexIndex = curIndex
            removeVertexItem.visible = (mapPolygon.count > 3 && menu._editingVertexIndex >= 0)
            menu.popup()
        }

        function popupCenter() {
            menu.popup()
        }

        QGCMenuItem {
            id:             removeVertexItem
            visible:        !_circleMode
            text:           qsTr("Remove vertex")
            onTriggered: {
                if (menu._editingVertexIndex >= 0) {
                    mapPolygon.removeVertex(menu._editingVertexIndex)
                }
            }
        }

        QGCMenuSeparator {
            visible:        removeVertexItem.visible
        }

        QGCMenuItem {
            text:           qsTr("Set radius..." )
            visible:        _circleMode
            onTriggered:    editCircleRadiusDialogFactory.open()
        }

        QGCMenuItem {
            text:           qsTr("Edit position..." )
            visible:        _circleMode
            onTriggered:    editCenterPositionDialogFactory.open()
        }

        QGCMenuItem {
            text:           qsTr("Edit position..." )
            visible:        !_circleMode && menu._editingVertexIndex >= 0
            onTriggered:    editVertexPositionDialogFactory.open()
        }
    }

    Component {
        id: polygonComponent

        MapPolygon {
            color:          mapPolygon.showAltColor ? altColor : interiorColor
            opacity:        interiorOpacity
            visible:        _root.visible
            border.color:   mapPolygon.vertexDrag ? "orange" : borderColor
            border.width:   mapPolygon.vertexDrag ? 3 : borderWidth
            path:           mapPolygon.vertexDrag ? mapPolygon.dragPath : mapPolygon.path
        }
    }

    Component {
        id: edgeLengthHandleComponent

        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  sourceItem.width / 2
            anchorPoint.y:  sourceItem.height / 2
            visible:        !_circleMode

            property int vertexIndex
            property real distance

            property var _unitsConversion: QGroundControl.unitsConversion

            sourceItem: Text {
              text:     _unitsConversion.metersToAppSettingsHorizontalDistanceUnits(distance).toFixed(1) + " " +
                        _unitsConversion.appSettingsHorizontalDistanceUnitsString
              color:    "white"
            }
        }
    }

    Component {
        id: edgeLengthHandlesComponent

        Repeater {
            model: _isVertexBeingDragged ? mapPolygon.path : undefined

            delegate: Item {
                property var _edgeLengthHandle

                function _setHandlePosition() {
                    var vertices = mapPolygon.vertexDrag ? mapPolygon.dragPath : mapPolygon.path
                    var nextIndex = index + 1
                    if (nextIndex > vertices.length - 1) {
                        nextIndex = 0
                    }
                    var distance = vertices[index].distanceTo(vertices[nextIndex])
                    var azimuth = vertices[index].azimuthTo(vertices[nextIndex])
                    _edgeLengthHandle.coordinate = vertices[index].atDistanceAndAzimuth(distance / 2, azimuth)
                    _edgeLengthHandle.distance = distance
                }

                Connections {
                    target: mapPolygon
                    function onDragPathChanged() { _setHandlePosition() }
                }

                Component.onCompleted: {
                    _edgeLengthHandle = edgeLengthHandleComponent.createObject(mapControl)
                    _edgeLengthHandle.vertexIndex = index
                    _setHandlePosition()
                    mapControl.addMapItem(_edgeLengthHandle)
                }

                Component.onDestruction: {
                    if (_edgeLengthHandle) {
                        _edgeLengthHandle.destroy()
                    }
                }
            }
        }
    }

    Component {
        id: splitHandleComponent

        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  sourceItem.width / 2
            anchorPoint.y:  sourceItem.height / 2
            visible:        !_circleMode && !_isVertexBeingDragged && !mapPolygon.centerDrag

            property int vertexIndex

            sourceItem: SplitIndicator {
                z:          _zorderSplitHandle
                onClicked:  if(_root.interactive) mapPolygon.splitPolygonSegment(mapQuickItem.vertexIndex)
            }
        }
    }

    Component {
        id: splitHandlesComponent

        Repeater {
            model: mapPolygon.path

            delegate: Item {
                property var _splitHandle
                property var _vertices:     mapPolygon.path

                function _setHandlePosition() {
                    var nextIndex = index + 1
                    if (nextIndex > _vertices.length - 1) {
                        nextIndex = 0
                    }
                    var distance = _vertices[index].distanceTo(_vertices[nextIndex])
                    var azimuth = _vertices[index].azimuthTo(_vertices[nextIndex])
                    _splitHandle.coordinate = _vertices[index].atDistanceAndAzimuth(distance / 2, azimuth)
                }

                Component.onCompleted: {
                    _splitHandle = splitHandleComponent.createObject(mapControl)
                    _splitHandle.vertexIndex = index
                    _setHandlePosition()
                    mapControl.addMapItem(_splitHandle)
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
            id:             dragArea
            mapControl:     _root.mapControl
            z:              _zorderDragHandle
            visible:        !_circleMode
            onDragStart:    { _isVertexBeingDragged = true; mapPolygon.vertexDrag = true }
            onDragStop:     { _isVertexBeingDragged = false; mapPolygon.vertexDrag = false; mapPolygon.verifyClockwiseWinding() }

            property int polygonVertex

            property bool _creationComplete: false

            Component.onCompleted: _creationComplete = true

            onItemCoordinateChanged: {
                if (_creationComplete) {
                    mapPolygon.adjustVertex(polygonVertex, itemCoordinate)
                }
            }

            onClicked: if(_root.interactive) menu.popupVertex(polygonVertex)
        }
    }

    Component {
        id: centerDragHandle
        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  dragHandle.width  * 0.5
            anchorPoint.y:  dragHandle.height * 0.5
            z:              _zorderDragHandle
            visible:        !_isVertexBeingDragged
            sourceItem: MapEditHandle {
                id:   dragHandle
                grip: true
            }
        }
    }

    Component {
        id: dragHandleComponent

        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  dragHandle.width  / 2
            anchorPoint.y:  dragHandle.height / 2
            z:              _zorderDragHandle
            visible:        !_circleMode && !mapPolygon.centerDrag

            property int polygonVertex

            sourceItem: MapEditHandle { id: dragHandle }
        }
    }

    Component {
        id: dragHandlesComponent

        Repeater {
            model: mapPolygon.pathModel

            delegate: Item {
                property var _visuals: [ ]

                Component.onCompleted: {
                    var dragHandle = dragHandleComponent.createObject(mapControl)
                    dragHandle.coordinate = Qt.binding(function() { return object.coordinate })
                    dragHandle.polygonVertex = Qt.binding(function() { return index })
                    mapControl.addMapItem(dragHandle)
                    var dragArea = dragAreaComponent.createObject(mapControl, { "itemIndicator": dragHandle, "itemCoordinate": object.coordinate })
                    dragArea.polygonVertex = Qt.binding(function() { return index })
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

    QGCPopupDialogFactory {
        id: editCircleRadiusDialogFactory

        dialogComponent: editCircleRadiusDialog
    }

    Component {
        id: editCircleRadiusDialog

        QGCPopupDialog {
            id:         popupDialog
            title:      qsTr("Set Radius")
            buttons:    Dialog.Save | Dialog.Cancel

            onAccepted: {
                const appRadius = Number(radiusField.text)
                if (!isNaN(appRadius) && appRadius > 0) {
                    const radiusMeters = QGroundControl.unitsConversion.appSettingsHorizontalDistanceUnitsToMeters(appRadius)
                    _createCircularPolygon(mapPolygon.center, radiusMeters)
                } else {
                    preventClose = true
                }
            }

            ColumnLayout {
                width:      ScreenTools.defaultFontPixelWidth * 30
                spacing:    ScreenTools.defaultFontPixelHeight

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               qsTr("Enter circle radius.")
                    wrapMode:           Text.WordWrap
                }

                QGCTextField {
                    id:                 radiusField
                    Layout.fillWidth:   true
                    text:               QGroundControl.unitsConversion.metersToAppSettingsHorizontalDistanceUnits(_circleRadius).toFixed(1)
                    validator:          DoubleValidator { bottom: 0.1; notation: DoubleValidator.StandardNotation }
                    inputMethodHints:   Qt.ImhFormattedNumbersOnly
                }

                QGCLabel {
                    Layout.fillWidth:   true
                    text:               QGroundControl.unitsConversion.appSettingsHorizontalDistanceUnitsString
                }
            }
        }
    }

    QGCPopupDialogFactory {
        id: editCenterPositionDialogFactory

        dialogComponent: editCenterPositionDialog
    }

    Component {
        id: editCenterPositionDialog

        EditPositionDialog {
            title:      qsTr("Edit Center Position")
            coordinate: mapPolygon.center
            onCoordinateChanged: {
                mapPolygon.centerDrag = true
                mapPolygon.center = coordinate
                mapPolygon.centerDrag = false
            }
        }
    }

    QGCPopupDialogFactory {
        id: editVertexPositionDialogFactory

        dialogComponent: editVertexPositionDialog
    }

    Component {
        id: editVertexPositionDialog

        EditPositionDialog {
            title:      qsTr("Edit Vertex Position")
            coordinate: mapPolygon.vertexCoordinate(menu._editingVertexIndex)
            onCoordinateChanged: {
                mapPolygon.adjustVertex(menu._editingVertexIndex, coordinate)
                mapPolygon.verifyClockwiseWinding()
            }
        }
    }

    Component {
        id: centerDragAreaComponent

        MissionItemIndicatorDrag {
            mapControl:                 _root.mapControl
            z:                          _zorderCenterHandle
            onItemCoordinateChanged:    mapPolygon.center = itemCoordinate
            onDragStart:                { mapPolygon.centerDrag = true; mapPolygon.vertexDrag = true }
            onDragStop:                 { mapPolygon.centerDrag = false; mapPolygon.vertexDrag = false }
            onClicked:                  if(_root.interactive) menu.popupCenter()
        }
    }

    Component {
        id: centerDragHandleComponent

        Item {
            property var dragHandle
            property var dragArea

            Component.onCompleted: {
                dragHandle = centerDragHandle.createObject(mapControl)
                dragHandle.coordinate = Qt.binding(function() { return mapPolygon.centerDrag ? mapPolygon.dragCenter : mapPolygon.center })
                mapControl.addMapItem(dragHandle)
                dragArea = centerDragAreaComponent.createObject(mapControl, { "itemIndicator": dragHandle, "itemCoordinate": mapPolygon.center })
            }

            Component.onDestruction: {
                dragHandle.destroy()
                dragArea.destroy()
            }
        }
    }

    Component {
        id: toolbarComponent

        PlanEditToolbar {
            mapControl:                     _root.mapControl
            caption:                        mapPolygon.traceMode ? _traceCaption() : _shapeCaption()
            accentEnabled:                  mapPolygon.traceComplete
            tools: mapPolygon.traceMode
                ? [ { text: qsTr("Cancel"), action: _cancelTrace },
                    { text: qsTr("Done"), accent: true, action: mapPolygon.finishTrace } ]
                : [ { text: qsTr("Rectangle"), action: _resetPolygon },
                    { text: qsTr("Circle"),    action: _resetCircle },
                    { separator: true },
                    { text: qsTr("Trace"),     action: _startTrace },
                    { text: qsTr("Import…"),   action: kmlOrSHPLoadDialog.openForLoad } ]
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
                    mapPolygon.appendVertex(mapControl.toCoordinate(Qt.point(mouse.x, mouse.y), false /* clipToViewPort */))
                }
            }
        }
    }

    Component {
        id: radiusDragHandleComponent

        MapQuickItem {
            id:             mapQuickItem
            anchorPoint.x:  dragHandle.width / 2
            anchorPoint.y:  dragHandle.height / 2
            z:              QGroundControl.zOrderMapItems + 2
            visible:        !mapPolygon.centerDrag

            sourceItem: MapEditHandle {
                id:      dragHandle
                opacity: interiorOpacity
            }
        }
    }

    Timer {
        id: radiusDragDebounceTimer
        interval: 0
        repeat: false

        property var pendingCoord: undefined

        onTriggered: {
            if (pendingCoord) {
                var coord = pendingCoord
                pendingCoord = undefined
                var radius = mapPolygon.center.distanceTo(coord)
                _createCircularPolygon(mapPolygon.center, radius)
            }
        }
    }

    Component {
        id: radiusDragAreaComponent

        MissionItemIndicatorDrag {
            mapControl: _root.mapControl
            onDragStart: {
                _circleRadiusDrag = true
                mapPolygon.vertexDrag = true
            }
            onDragStop: {
                _circleRadiusDrag = false
                mapPolygon.vertexDrag = false
            }

            onItemCoordinateChanged: {
                // Keep the handle visually attached to the cursor while radius updates are de-bounced.
                _circleRadiusDragCoord = itemCoordinate

                var radius = mapPolygon.center.distanceTo(itemCoordinate)

                if (Math.abs(radius - _circleRadius) > 0.1) {
                    radiusDragDebounceTimer.pendingCoord = itemCoordinate
                    radiusDragDebounceTimer.start()
                }
            }
        }
    }

    Component {
        id: radiusVisualsComponent

        Item {
            property var    circleCenterCoord:  mapPolygon.center

            function _calcRadiusDragCoord() {
                _circleRadiusDragCoord = circleCenterCoord.atDistanceAndAzimuth(_circleRadius, 90)
            }

            onCircleCenterCoordChanged: {
                if (!_circleRadiusDrag) {
                    _calcRadiusDragCoord()
                }
            }

            QGCDynamicObjectManager {
                id: _objMgr
            }

            Component.onCompleted: {
                _calcRadiusDragCoord()
                var radiusDragHandle = _objMgr.createObject(radiusDragHandleComponent, mapControl, true)
                radiusDragHandle.coordinate = Qt.binding(function() { return _circleRadiusDragCoord })
                var radiusDragIndicator = radiusDragAreaComponent.createObject(mapControl, { "itemIndicator": radiusDragHandle, "itemCoordinate": _circleRadiusDragCoord })
                _objMgr.addObject(radiusDragIndicator)
            }
        }
    }
}
