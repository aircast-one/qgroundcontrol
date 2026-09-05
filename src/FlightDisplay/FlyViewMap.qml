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
import QtQuick.Layouts

import QGroundControl
import QGroundControl.Controllers
import QGroundControl.Controls
import QGroundControl.FlightDisplay
import QGroundControl.FlightMap
import QGroundControl.Palette
import QGroundControl.ScreenTools
import QGroundControl.Vehicle

FlightMap {
    id:                         _root
    allowGCSLocationCenter:     true
    allowVehicleLocationCenter: !_keepVehicleCentered
    planView:                   false
    zoomLevel:                  QGroundControl.flightMapZoom
    center:                     QGroundControl.flightMapPosition
    panEnabled:                 !globals.overlayRigFlyView || !globals.overlayRigFlyView.editMode

    property Item   pipView
    property Item   pipState:                   _pipState
    property var    rightPanelWidth
    property var    planMasterController
    property bool   pipMode:                    false
    property var    toolInsets

    property var    _activeVehicle:             QGroundControl.multiVehicleManager.activeVehicle
    property var    _planMasterController:      planMasterController
    property var    _geoFenceController:        planMasterController.geoFenceController
    property var    _rallyPointController:      planMasterController.rallyPointController
    property var    _activeVehicleCoordinate:   _activeVehicle ? _activeVehicle.coordinate : QtPositioning.coordinate()
    property real   _toolButtonTopMargin:       parent.height - mainWindow.height + (ScreenTools.defaultFontPixelHeight / 2)
    property real   _toolsMargin:               ScreenTools.defaultFontPixelWidth * 0.75
    property var    _flyViewSettings:           QGroundControl.settingsManager.flyViewSettings
    property bool   _keepMapCenteredOnVehicle:  _flyViewSettings.keepMapCenteredOnVehicle.rawValue

    property bool   _disableVehicleTracking:    false
    property bool   _keepVehicleCentered:       pipMode ? true : false
    property bool   _saveZoomLevelSetting:      true

    function _adjustMapZoomForPipMode() {
        _saveZoomLevelSetting = false
        if (pipMode) {
            if (QGroundControl.flightMapZoom > 3) {
                zoomLevel = QGroundControl.flightMapZoom - 3
            }
        } else {
            zoomLevel = QGroundControl.flightMapZoom
        }
        _saveZoomLevelSetting = true
    }

    onPipModeChanged: _adjustMapZoomForPipMode()

    onVisibleChanged: {
        if (visible) {
            center = QGroundControl.flightMapPosition
        }
    }

    onZoomLevelChanged: {
        if (_saveZoomLevelSetting) {
            QGroundControl.flightMapZoom = _root.zoomLevel
        }
    }
    onCenterChanged: {
        QGroundControl.flightMapPosition = _root.center
    }

    onMapPanStart:  _disableVehicleTracking = true
    onMapPanStop:   panRecenterTimer.restart()

    function pointInRect(point, rect) {
        return point.x > rect.x &&
                point.x < rect.x + rect.width &&
                point.y > rect.y &&
                point.y < rect.y + rect.height;
    }

    property real _animatedLatitudeStart
    property real _animatedLatitudeStop
    property real _animatedLongitudeStart
    property real _animatedLongitudeStop
    property real animatedLatitude
    property real animatedLongitude

    onAnimatedLatitudeChanged: _root.center = QtPositioning.coordinate(animatedLatitude, animatedLongitude)
    onAnimatedLongitudeChanged: _root.center = QtPositioning.coordinate(animatedLatitude, animatedLongitude)

    NumberAnimation on animatedLatitude { id: animateLat; from: _animatedLatitudeStart; to: _animatedLatitudeStop; duration: 1000 }
    NumberAnimation on animatedLongitude { id: animateLong; from: _animatedLongitudeStart; to: _animatedLongitudeStop; duration: 1000 }

    function animatedMapRecenter(fromCoord, toCoord) {
        _animatedLatitudeStart = fromCoord.latitude
        _animatedLongitudeStart = fromCoord.longitude
        _animatedLatitudeStop = toCoord.latitude
        _animatedLongitudeStop = toCoord.longitude
        animateLat.start()
        animateLong.start()
    }

    function _insetCenterRect() {
        return Qt.rect(toolInsets.leftEdgeCenterInset,
                       toolInsets.topEdgeCenterInset,
                       _root.width - toolInsets.leftEdgeCenterInset - toolInsets.rightEdgeCenterInset,
                       _root.height - toolInsets.topEdgeCenterInset - toolInsets.bottomEdgeCenterInset)
    }

    function _insetCornerRects() {
        var rects = {
        "topleft":      Qt.rect(0,0,
                               toolInsets.leftEdgeTopInset,
                               toolInsets.topEdgeLeftInset),
        "topright":     Qt.rect(_root.width-toolInsets.rightEdgeTopInset,0,
                               toolInsets.rightEdgeTopInset,
                               toolInsets.topEdgeRightInset),
        "bottomleft":   Qt.rect(0,_root.height-toolInsets.bottomEdgeLeftInset,
                               toolInsets.leftEdgeBottomInset,
                               toolInsets.bottomEdgeLeftInset),
        "bottomright":  Qt.rect(_root.width-toolInsets.rightEdgeBottomInset,_root.height-toolInsets.bottomEdgeRightInset,
                               toolInsets.rightEdgeBottomInset,
                               toolInsets.bottomEdgeRightInset)}
        return rects
    }

    function recenterNeeded() {
        var vehiclePoint = _root.fromCoordinate(_activeVehicleCoordinate, false)
        var centerRect = _insetCenterRect()

        if(!pointInRect(vehiclePoint, centerRect)){
            return true
        }

        var cornerRects = _insetCornerRects()
        if(pointInRect(vehiclePoint, cornerRects["topleft"])){
            return true
        } else if(pointInRect(vehiclePoint, cornerRects["topright"])){
            return true
        } else if(pointInRect(vehiclePoint, cornerRects["bottomleft"])){
            return true
        } else if(pointInRect(vehiclePoint, cornerRects["bottomright"])){
            return true
        }

        return false
    }

    function updateMapToVehiclePosition() {
        if (animateLat.running || animateLong.running) {
            return
        }
        if (!_keepMapCenteredOnVehicle && firstVehiclePositionReceived && _activeVehicleCoordinate.isValid && !_disableVehicleTracking) {
            if (_keepVehicleCentered) {
                _root.center = _activeVehicleCoordinate
            } else {
                if (firstVehiclePositionReceived && recenterNeeded()) {
                    var vehiclePoint = _root.fromCoordinate(_activeVehicleCoordinate, false)
                    var centerInsetRect = _insetCenterRect()
                    var centerInsetPoint = Qt.point(centerInsetRect.x + centerInsetRect.width / 2, centerInsetRect.y + centerInsetRect.height / 2)
                    var centerOffset = Qt.point((_root.width / 2) - centerInsetPoint.x, (_root.height / 2) - centerInsetPoint.y)
                    var vehicleOffsetPoint = Qt.point(vehiclePoint.x + centerOffset.x, vehiclePoint.y + centerOffset.y)
                    var vehicleOffsetCoord = _root.toCoordinate(vehicleOffsetPoint, false)
                    animatedMapRecenter(_root.center, vehicleOffsetCoord)
                }
            }
        }
    }

    on_ActiveVehicleCoordinateChanged: {
        if (_keepMapCenteredOnVehicle && _activeVehicleCoordinate.isValid && !_disableVehicleTracking) {
            _root.center = _activeVehicleCoordinate
        }
    }

    PipState {
        id:         _pipState
        pipView:    _root.pipView
        isDark:     _isFullWindowItemDark
    }

    Timer {
        id:         panRecenterTimer
        interval:   10000
        running:    false
        onTriggered: {
            _disableVehicleTracking = false
            updateMapToVehiclePosition()
        }
    }

    Timer {
        interval:       500
        running:        true
        repeat:         true
        onTriggered:    updateMapToVehiclePosition()
    }

    QGCMapPalette { id: mapPal; lightColors: isSatelliteMap }

    Connections {
        target:                 _missionController
        ignoreUnknownSignals:   true
        function onNewItemsFromVehicle() {
            var visualItems = _missionController.visualItems
            if (visualItems && visualItems.count !== 1) {
                mapFitFunctions.fitMapViewportToMissionItems()
                firstVehiclePositionReceived = true
            }
        }
    }

    MapFitFunctions {
        id:                         mapFitFunctions
        map:                        _root
        usePlannedHomePosition:     false
        planMasterController:       _planMasterController
    }

    ObstacleDistanceOverlayMap {
        id: obstacleDistance
        showText: !pipMode
    }

    MapPolyline {
        id:         trajectoryPolyline
        line.width: 3
        line.color: "red"
        z:          QGroundControl.zOrderTrajectoryLines
        visible:    !pipMode

        Connections {
            target:                 QGroundControl.multiVehicleManager
            function onActiveVehicleChanged(activeVehicle) {
                trajectoryPolyline.path = _activeVehicle ? _activeVehicle.trajectoryPoints.list() : []
            }
        }

        Connections {
            target:                             _activeVehicle ? _activeVehicle.trajectoryPoints : null
            onPointAdded: (coordinate) =>       trajectoryPolyline.addCoordinate(coordinate)
            onUpdateLastPoint: (coordinate) =>  trajectoryPolyline.replaceCoordinate(trajectoryPolyline.pathLength() - 1, coordinate)
            onPointsCleared:                    trajectoryPolyline.path = []
        }
    }

    MapItemView {
        model: QGroundControl.multiVehicleManager.vehicles
        delegate: VehicleMapItem {
            vehicle:        object
            coordinate:     object.coordinate
            map:            _root
            size:           pipMode ? ScreenTools.defaultFontPixelHeight : ScreenTools.defaultFontPixelHeight * 3
            z:              QGroundControl.zOrderVehicles
        }
    }
    MapItemView{
        model: QGroundControl.multiVehicleManager.vehicles
        delegate: ProximityRadarMapView {
            vehicle:        object
            coordinate:     object.coordinate
            map:            _root
            z:              QGroundControl.zOrderVehicles
        }
    }
    MapItemView {
        model: QGroundControl.adsbVehicleManager.adsbVehicles
        delegate: VehicleMapItem {
            coordinate:     object.coordinate
            altitude:       object.altitude
            callsign:       object.callsign
            heading:        object.heading
            alert:          object.alert
            map:            _root
            size:           pipMode ? ScreenTools.defaultFontPixelHeight : ScreenTools.defaultFontPixelHeight * 2.5
            z:              QGroundControl.zOrderVehicles
        }
    }

    Repeater {
        model: QGroundControl.multiVehicleManager.vehicles

        PlanMapItems {
            map:                    _root
            largeMapView:           !pipMode
            planMasterController:   masterController
            vehicle:                _vehicle

            property var _vehicle: object

            PlanMasterController {
                id: masterController
                Component.onCompleted: startStaticActiveVehicle(object)
            }
        }
    }

    CustomMapItems {
        map:            _root
        largeMapView:   !pipMode
    }

    GeoFenceMapVisuals {
        map:                    _root
        myGeoFenceController:   _geoFenceController
        interactive:            false
        planView:               false
        homePosition:           _activeVehicle && _activeVehicle.homePosition.isValid ? _activeVehicle.homePosition :  QtPositioning.coordinate()
    }

    MapItemView {
        model: _rallyPointController.points

        delegate: MapQuickItem {
            id:             itemIndicator
            anchorPoint.x:  sourceItem.anchorPointX
            anchorPoint.y:  sourceItem.anchorPointY
            coordinate:     object.coordinate
            z:              QGroundControl.zOrderMapItems

            sourceItem: MissionItemIndexLabel {
                id:         itemIndexLabel
                label:      qsTr("R", "rally point map item label")
            }
        }
    }

    MapItemView {
        model: _activeVehicle ? _activeVehicle.cameraTriggerPoints : 0

        delegate: CameraTriggerIndicator {
            coordinate:     object.coordinate
            z:              QGroundControl.zOrderTopMost
        }
    }

    QGCMapCircleVisuals {
        id:                 fwdFlightGotoMapCircle
        mapControl:         parent
        mapCircle:          _fwdFlightGotoMapCircle
        radiusLabelVisible: true
        visible:            gotoLocationItem.visible && _activeVehicle &&
                            _activeVehicle.inFwdFlight &&
                            !_activeVehicle.orbitActive

        property alias coordinate: _fwdFlightGotoMapCircle.center
        property alias radius: _fwdFlightGotoMapCircle.radius

        Component.onCompleted: {
            centerDragHandleVisible = false

            globals.guidedControllerFlyView.fwdFlightGotoMapCircle = this
        }

        Binding {
            target: _fwdFlightGotoMapCircle
            property: "center"
            value: gotoLocationItem.coordinate
        }

        function startLoiterRadiusEdit() {
            _fwdFlightGotoMapCircle.interactive = true
        }

        function actionConfirmed() {
            _fwdFlightGotoMapCircle.interactive = false
            _fwdFlightGotoMapCircle._commitRadius()
        }

        function actionCancelled() {
            _fwdFlightGotoMapCircle.interactive = false
            _fwdFlightGotoMapCircle._restoreRadius()
        }

        QGCMapCircle {
            id:                 _fwdFlightGotoMapCircle
            interactive:        false
            showRotation:       true
            clockwiseRotation:  true

            property real _defaultLoiterRadius: _flyViewSettings.forwardFlightGoToLocationLoiterRad.value
            property real _committedRadius;

            onCenterChanged: {
                radius.rawValue = _defaultLoiterRadius
            }

            Component.onCompleted: {
                radius.rawValue = _defaultLoiterRadius
                _commitRadius()
            }

            function _commitRadius() {
                _committedRadius = radius.rawValue
            }

            function _restoreRadius() {
                radius.rawValue = _committedRadius
            }
        }
    }

    MapQuickItem {
        id:             gotoLocationItem
        visible:        false
        z:              QGroundControl.zOrderMapItems
        anchorPoint.x:  sourceItem.anchorPointX
        anchorPoint.y:  sourceItem.anchorPointY
        sourceItem: MissionItemIndexLabel {
            checked:    true
            index:      -1
            label:      qsTr("Go here", "Go to location waypoint")
        }

        property bool inGotoFlightMode: _activeVehicle ? _activeVehicle.flightMode === _activeVehicle.gotoFlightMode : false

        property var _committedCoordinate: null

        onInGotoFlightModeChanged: {
            if (!inGotoFlightMode && gotoLocationItem.visible) {
                hide()
            }
        }

        function show(coord) {
            gotoLocationItem.coordinate = coord
            gotoLocationItem.visible = true
        }

        function hide() {
            gotoLocationItem.visible = false
        }

        function actionConfirmed() {
            _commitCoordinate()

            fwdFlightGotoMapCircle.actionConfirmed()

        }

        function actionCancelled() {
            _restoreCoordinate()

            fwdFlightGotoMapCircle.actionCancelled()
        }

        function _commitCoordinate() {
            _committedCoordinate = QtPositioning.coordinate(
                coordinate.latitude,
                coordinate.longitude
            );
        }

        function _restoreCoordinate() {
            if (_committedCoordinate) {
                coordinate = _committedCoordinate
            } else {
                hide()
            }
        }
    }

    QGCMapCircleVisuals {
        id:             orbitMapCircle
        mapControl:     parent
        mapCircle:      _mapCircle
        visible:        false

        property alias center:              _mapCircle.center
        property alias clockwiseRotation:   _mapCircle.clockwiseRotation
        readonly property real defaultRadius: 30

        Connections {
            target: QGroundControl.multiVehicleManager
            function onActiveVehicleChanged(activeVehicle) {
                if (!activeVehicle) {
                    orbitMapCircle.visible = false
                }
            }
        }

        function show(coord) {
            _mapCircle.radius.rawValue = defaultRadius
            orbitMapCircle.center = coord
            orbitMapCircle.visible = true
        }

        function hide() {
            orbitMapCircle.visible = false
        }

        function actionConfirmed() {
            hide()
        }

        function actionCancelled() {
            hide()
        }

        function radius() {
            return _mapCircle.radius.rawValue
        }

        Component.onCompleted: globals.guidedControllerFlyView.orbitMapCircle = orbitMapCircle

        QGCMapCircle {
            id:                 _mapCircle
            interactive:        true
            radius.rawValue:    30
            showRotation:       true
            clockwiseRotation:  true
        }
    }

    MapQuickItem {
        id:             roiLocationItem
        visible:        _activeVehicle && _activeVehicle.isROIEnabled
        z:              QGroundControl.zOrderMapItems
        anchorPoint.x:  sourceItem.anchorPointX
        anchorPoint.y:  sourceItem.anchorPointY

        Connections {
            target: _activeVehicle
            onRoiCoordChanged: (centerCoord) => {
                roiLocationItem.show(centerCoord)
            }
        }

        MouseArea {
            anchors.fill: parent
            onClicked: (position) => {
                position = Qt.point(position.x, position.y)
                var clickCoord = _root.toCoordinate(position, false)
                position = mapToItem(globals.parent, position)
                var dropPanel = roiEditDropPanelComponent.createObject(mainWindow, { clickRect: Qt.rect(position.x, position.y, 0, 0) })
                dropPanel.open()
            }
        }

        sourceItem: MissionItemIndexLabel {
            checked:    true
            index:      -1
            label:      qsTr("ROI here", "Make this a Region Of Interest")
        }

        function show(coord) {
            roiLocationItem.coordinate = coord
        }
    }

    QGCMapCircleVisuals {
        id:             orbitTelemetryCircle
        mapControl:     parent
        mapCircle:      _activeVehicle ? _activeVehicle.orbitMapCircle : null
        visible:        _activeVehicle ? _activeVehicle.orbitActive : false
    }

    MapQuickItem {
        id:             orbitCenterIndicator
        anchorPoint.x:  sourceItem.anchorPointX
        anchorPoint.y:  sourceItem.anchorPointY
        coordinate:     _activeVehicle ? _activeVehicle.orbitMapCircle.center : QtPositioning.coordinate()
        visible:        orbitTelemetryCircle.visible && !gotoLocationItem.visible

        sourceItem: MissionItemIndexLabel {
            checked:    true
            index:      -1
            label:      qsTr("Orbit", "Orbit waypoint")
        }
    }

    Component {
        id: roiEditPositionDialogComponent

        EditPositionDialog {
            title:                  qsTr("Edit ROI Position")
            coordinate:             roiLocationItem.coordinate
            onCoordinateChanged: {
                roiLocationItem.coordinate = coordinate
                _activeVehicle.guidedModeROI(coordinate)
            }
        }
    }

    Component {
        id: roiEditDropPanelComponent

        DropPanel {
            id: roiEditDropPanel

            sourceComponent: Component {
                ColumnLayout {
                    spacing: ScreenTools.defaultFontPixelWidth / 2

                    OverlayMenuItem {
                        text:               qsTr("Cancel ROI")
                        onClicked: {
                            _activeVehicle.stopGuidedModeROI()
                            roiEditDropPanel.close()
                        }
                    }

                    OverlayMenuItem {
                        text:               qsTr("Edit Position")
                        onClicked: {         
                            roiEditPositionDialogComponent.createObject(mainWindow, { showSetPositionFromVehicle: false }).open()
                            roiEditDropPanel.close()
                        }
                    }
                }
            }
        }
    }

    Component {
        id: mapClickDropPanelComponent

        DropPanel {
            id: mapClickDropPanel

            property var mapClickCoord

            sourceComponent: Component {
                ColumnLayout {
                    spacing: ScreenTools.defaultFontPixelWidth / 2

                    property bool _showGoto:    globals.guidedControllerFlyView.showGotoLocation
                    property bool _showOrbit:   globals.guidedControllerFlyView.showOrbit
                    property bool _showROI:     globals.guidedControllerFlyView.showROI
                    property bool _showHome:    globals.guidedControllerFlyView.showSetHome
                    property bool _showHeading: globals.guidedControllerFlyView.showChangeHeading
                    property bool _showOrigin:  globals.guidedControllerFlyView.showSetEstimatorOrigin

                    OverlayMenuItem {
                        text:               qsTr("Go to location")
                        visible:            _showGoto
                        onClicked: {
                            mapClickDropPanel.close()
                            gotoLocationItem.show(mapClickCoord)

                            if ((_activeVehicle.flightMode == _activeVehicle.gotoFlightMode) && !_flyViewSettings.goToLocationRequiresConfirmInGuided.value) {
                                globals.guidedControllerFlyView.executeAction(globals.guidedControllerFlyView.actionGoto, mapClickCoord, gotoLocationItem)
                                gotoLocationItem.actionConfirmed()
                            } else {
                                globals.guidedControllerFlyView.confirmAction(globals.guidedControllerFlyView.actionGoto, mapClickCoord, gotoLocationItem)
                            }
                        }
                    }

                    OverlayMenuSeparator { visible: _showOrbit && _showGoto }

                    OverlayMenuItem {
                        text:               qsTr("Orbit at location")
                        visible:            _showOrbit
                        onClicked: {
                            mapClickDropPanel.close()
                            orbitMapCircle.show(mapClickCoord)
                            globals.guidedControllerFlyView.confirmAction(globals.guidedControllerFlyView.actionOrbit, mapClickCoord, orbitMapCircle)
                        }
                    }

                    OverlayMenuSeparator { visible: _showROI && (_showGoto || _showOrbit) }

                    OverlayMenuItem {
                        text:               qsTr("ROI at location")
                        visible:            _showROI
                        onClicked: {
                            mapClickDropPanel.close()
                            globals.guidedControllerFlyView.executeAction(globals.guidedControllerFlyView.actionROI, mapClickCoord, 0, false)
                        }
                    }

                    OverlayMenuSeparator { visible: _showHome && (_showGoto || _showOrbit || _showROI) }

                    OverlayMenuItem {
                        text:               qsTr("Set home here")
                        visible:            _showHome
                        onClicked: {
                            mapClickDropPanel.close()
                            globals.guidedControllerFlyView.confirmAction(globals.guidedControllerFlyView.actionSetHome, mapClickCoord)
                        }
                    }

                    OverlayMenuSeparator { visible: _showHeading && (_showGoto || _showOrbit || _showROI || _showHome) }

                    OverlayMenuItem {
                        text:               qsTr("Set Heading")
                        visible:            _showHeading
                        onClicked: {
                            mapClickDropPanel.close()
                            globals.guidedControllerFlyView.confirmAction(globals.guidedControllerFlyView.actionChangeHeading, mapClickCoord)
                        }
                    }

                    OverlayMenuSeparator { visible: _showOrigin && (_showGoto || _showOrbit || _showROI || _showHome || _showHeading) }

                    OverlayMenuItem {
                        text:               qsTr("Set Estimator Origin")
                        visible:            _showOrigin
                        opacity:            0.7
                        onClicked: {
                            mapClickDropPanel.close()
                            globals.guidedControllerFlyView.confirmAction(globals.guidedControllerFlyView.actionSetEstimatorOrigin, mapClickCoord)
                        }
                    }

                    OverlayMenuSeparator { }

                    ColumnLayout {
                        Layout.alignment:   Qt.AlignHCenter
                        spacing:            0

                        QGCLabel {
                            Layout.alignment:   Qt.AlignHCenter
                            opacity:            0.6
                            font.pointSize:     ScreenTools.smallFontPointSize
                            text:               qsTr("Lat: %1").arg(mapClickCoord.latitude.toFixed(6))
                        }

                        QGCLabel {
                            Layout.alignment:   Qt.AlignHCenter
                            opacity:            0.6
                            font.pointSize:     ScreenTools.smallFontPointSize
                            text:               qsTr("Lon: %1").arg(mapClickCoord.longitude.toFixed(6))
                        }
                    }
                }
            }
        }
    }

    property var _mapClickMenuCoord: null

    MapQuickItem {
        coordinate:     _mapClickMenuCoord ? _mapClickMenuCoord : QtPositioning.coordinate()
        visible:        _mapClickMenuCoord !== null
        z:              QGroundControl.zOrderMapItems + 1
        anchorPoint.x:  clickMarker.width / 2
        anchorPoint.y:  clickMarker.height / 2

        sourceItem: Item {
            id:     clickMarker
            width:  ScreenTools.defaultFontPixelHeight * 1.6
            height: width

            Rectangle {
                anchors.fill:   parent
                radius:         width / 2
                color:          "transparent"
                border.color:   Qt.rgba(0, 0, 0, 0.6)
                border.width:   4
            }

            Rectangle {
                anchors.fill:       parent
                anchors.margins:    1
                radius:             width / 2
                color:              "transparent"
                border.color:       "white"
                border.width:       2
            }

            Rectangle {
                anchors.centerIn:   parent
                width:              4
                height:             4
                radius:             2
                color:              "white"
            }
        }
    }

    onMapRightClicked: (position) => {
        if (!globals.guidedControllerFlyView.guidedUIVisible &&
            (globals.guidedControllerFlyView.showGotoLocation || globals.guidedControllerFlyView.showOrbit ||
             globals.guidedControllerFlyView.showROI || globals.guidedControllerFlyView.showSetHome ||
             globals.guidedControllerFlyView.showChangeHeading ||
             globals.guidedControllerFlyView.showSetEstimatorOrigin)) {

            position = Qt.point(position.x, position.y)
            var clickCoord = _root.toCoordinate(position, false)
            position = _root.mapToItem(globals.parent, position)
            var dropPanel = mapClickDropPanelComponent.createObject(mainWindow, { mapClickCoord: clickCoord, clickRect: Qt.rect(position.x, position.y, 0, 0) })
            _mapClickMenuCoord = clickCoord
            dropPanel.closed.connect(function() {
                _mapClickMenuCoord = null
                dropPanel.destroy()
            })
            dropPanel.open()
        }
    }

    MapScale {
        id:                 mapScale
        anchors.margins:    _toolsMargin
        anchors.left:       parent.left
        anchors.top:        parent.top
        mapControl:         _root
        buttonsOnLeft:      true
        visible:            !ScreenTools.isTinyScreen && QGroundControl.corePlugin.options.flyView.showMapScale && mapControl.pipState.state === mapControl.pipState.windowState

        property real centerInset: visible ? parent.height - y : 0
    }

}
