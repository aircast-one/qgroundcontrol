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
import QtQuick.Dialogs
import QtQuick.Effects
import QtLocation
import QtPositioning
import QtQuick.Layouts
import QtQuick.Window

import QGroundControl
import QGroundControl.FlightMap
import QGroundControl.ScreenTools
import QGroundControl.Controls
import QGroundControl.FactSystem
import QGroundControl.FactControls
import QGroundControl.Palette
import QGroundControl.Controllers
import QGroundControl.ShapeFileHelper
import QGroundControl.FlightDisplay
import QGroundControl.UTMSP


Item {
    id: _root

    property bool planControlColapsed: false

    // The elevation profile is a drawer, not a permanent strip. It used to occupy its height
    // whenever the setting was on, and the map's centre viewport was computed from where it
    // happened to sit.
    property bool _terrainProfileOpen:            _planViewSettings.showMissionItemStatus.rawValue &&
                                                      _editingLayer === _layerMission &&
                                                      QGroundControl.corePlugin.options.showMissionStatus &&
                                                      // An empty plan has no profile to draw, and a
                                                      // sheet of empty axes is a panel asking to be
                                                      // read for nothing.
                                                      _planMasterController.containsItems
    readonly property real _terrainProfileHeight: ScreenTools.defaultFontPixelHeight * 8.5

    readonly property int   _decimalPlaces:             8
    readonly property real  _margin:                    ScreenTools.defaultFontPixelHeight * 0.5
    readonly property real  _toolsMargin:               ScreenTools.defaultFontPixelWidth * 0.75
    readonly property real  _radius:                    ScreenTools.defaultFontPixelWidth  * 0.5
    // Derived, not branched. The width used to be an if/else that added 21.667 characters when
    // UTMSP was on - a number with no relationship to anything on screen.
    readonly property real  _rightPanelWidth:           Math.min(width / 3, ScreenTools.defaultFontPixelWidth * (_utmspEnabled ? 52 : 40))
    // Everything floats clear of the window edge by the same amount, so the map reads as one
    // surface with panels resting on it rather than as a set of docked regions butted together.
    readonly property real  _panelMargin:               ScreenTools.defaultFontPixelHeight * 0.9
    readonly property real  _panelRadius:               ScreenTools.defaultFontPixelHeight * 1.3
    readonly property var   _defaultVehicleCoordinate:  QtPositioning.coordinate(37.803784, -122.462276)
    readonly property bool  _waypointsOnlyMode:         QGroundControl.corePlugin.options.missionWaypointsOnly

    property var    _planMasterController:              planMasterController
    property var    _missionController:                 _planMasterController.missionController
    property var    _geoFenceController:                _planMasterController.geoFenceController
    property var    _rallyPointController:              _planMasterController.rallyPointController
    property var    _visualItems:                       _missionController.visualItems
    property bool   _lightWidgetBorders:                editorMap.isSatelliteMap
    property bool   _addROIOnClick:                     false
    // The add-waypoint mode used to live as `checked` on a ToolStripAction, so the map's click
    // handler reached into the tool strip's model to find out what a click meant. The mode is
    // the view's state; the dock button only reflects it.
    property bool   _addWaypointMode:                   false
    property bool   _singleComplexItem:                 _missionController.complexMissionItemNames.length === 1
    // One index over one list. There used to be two tab bars with duplicated buttons and two
    // parallel expressions selecting between them, so adding a layer meant editing both.
    property int    _layerIndex:                        0
    readonly property var _activeLayers:                _utmspEnabled ? _layersUTMSP : _layers
    property int    _editingLayer:                      _activeLayers[Math.min(_layerIndex, _activeLayers.length - 1)]
    property var    _appSettings:                       QGroundControl.settingsManager.appSettings
    property var    _planViewSettings:                  QGroundControl.settingsManager.planViewSettings
    property bool   _promptForPlanUsageShowing:         false
    property bool   _utmspEnabled:                      QGroundControl.utmspSupported
    property bool   _resetGeofencePolygon:              false   //Reset the Geofence Polygon
    property var    _vehicleID
    property bool   _triggerSubmit
    property bool   _resetRegisterFlightPlan

    readonly property var       _layers:                    [_layerMission, _layerGeoFence, _layerRallyPoints]
    readonly property var       _layersUTMSP:               [_layerMission, _layerRallyPoints, _layerUTMSP] //Adds additional UTMSP layer

    readonly property int       _layerMission:              1
    readonly property int       _layerGeoFence:             2
    readonly property int       _layerRallyPoints:          3
    readonly property int       _layerUTMSP:                4 // Additional Tab button when UTMSP is enabled
    readonly property string    _armedVehicleUploadPrompt:  qsTr("Vehicle is currently armed. Do you want to upload the mission to the vehicle?")

    readonly property var       _qgcPal:                    QGroundControl.globalPalette

    // A complex item (survey, corridor, structure scan, landing pattern) carries far more than a
    // row can hold, so selecting one pushes a detail page instead of expanding in place.
    readonly property var  _currentPlanItem: _missionController.currentPlanViewItem
    readonly property bool _showItemDetail:  _currentPlanItem !== null && _currentPlanItem !== undefined &&
                                                 !_currentPlanItem.isSimpleItem && _currentPlanItem.sequenceNumber !== 0

    function allAddClickBoolsOff() {
        _addROIOnClick   = false
        _addWaypointMode = false
    }


    function mapCenter() {
        var coordinate = editorMap.center
        coordinate.latitude  = coordinate.latitude.toFixed(_decimalPlaces)
        coordinate.longitude = coordinate.longitude.toFixed(_decimalPlaces)
        coordinate.altitude  = coordinate.altitude.toFixed(_decimalPlaces)
        return coordinate
    }

    property bool _firstMissionLoadComplete:    false
    property bool _firstFenceLoadComplete:      false
    property bool _firstRallyLoadComplete:      false
    property bool _firstLoadComplete:           false

    MapFitFunctions {
        id:                         mapFitFunctions  // The name for this id cannot be changed without breaking references outside of this code. Beware!
        map:                        editorMap
        usePlannedHomePosition:     true
        planMasterController:       _planMasterController
    }

    onVisibleChanged: {
        if(visible) {
            editorMap.zoomLevel = QGroundControl.flightMapZoom
            editorMap.center    = QGroundControl.flightMapPosition
            OverlayBackdrop.refresh()
            if (!_planMasterController.containsItems) {
                fileMenu.openFrom(fileButton)
            }
        }
    }

    Connections {
        target: _appSettings ? _appSettings.defaultMissionItemAltitude : null
        function onRawValueChanged() {
            if (_visualItems.count > 1) {
                mainWindow.showMessageDialog(qsTr("Apply new altitude"),
                                             qsTr("You have changed the default altitude for mission items. Would you like to apply that altitude to all the items in the current mission?"),
                                             Dialog.Yes | Dialog.No,
                                             function() { _missionController.applyDefaultMissionAltitude() })
            }
        }
    }

    Component {
        id: promptForPlanUsageOnVehicleChangePopupComponent
        QGCPopupDialog {
            title:      _planMasterController.managerVehicle.isOfflineEditingVehicle ? qsTr("Plan View - Vehicle Disconnected") : qsTr("Plan View - Vehicle Changed")
            buttons:    Dialog.NoButton

            ColumnLayout {
                QGCLabel {
                    Layout.maximumWidth:    parent.width
                    wrapMode:               QGCLabel.WordWrap
                    text:                   _planMasterController.managerVehicle.isOfflineEditingVehicle ?
                                                qsTr("The vehicle associated with the plan in the Plan View is no longer available. What would you like to do with that plan?") :
                                                qsTr("The plan being worked on in the Plan View is not from the current vehicle. What would you like to do with that plan?")
                }

                QGCButton {
                    Layout.fillWidth:   true
                    text:               _planMasterController.dirty ?
                                            (_planMasterController.managerVehicle.isOfflineEditingVehicle ?
                                                 qsTr("Discard Unsaved Changes") :
                                                 qsTr("Discard Unsaved Changes, Load New Plan From Vehicle")) :
                                            qsTr("Load New Plan From Vehicle")
                    onClicked: {
                        _planMasterController.showPlanFromManagerVehicle()
                        _promptForPlanUsageShowing = false
                        close();
                    }
                }

                QGCButton {
                    Layout.fillWidth:   true
                    text:               _planMasterController.managerVehicle.isOfflineEditingVehicle ?
                                            qsTr("Keep Current Plan") :
                                            qsTr("Keep Current Plan, Don't Update From Vehicle")
                    onClicked: {
                        if (!_planMasterController.managerVehicle.isOfflineEditingVehicle) {
                            _planMasterController.dirty = true
                        }
                        _promptForPlanUsageShowing = false
                        close()
                    }
                }
            }
        }
    }

    PlanMasterController {
        id:         planMasterController
        flyView:    false

        Component.onCompleted: {
            _planMasterController.start()
            _missionController.setCurrentPlanViewSeqNum(0, true)
        }

        onPromptForPlanUsageOnVehicleChange: {
            if (!_promptForPlanUsageShowing) {
                _promptForPlanUsageShowing = true
                promptForPlanUsageOnVehicleChangePopupComponent.createObject(mainWindow).open()
            }
        }

        function waitingOnIncompleteDataMessage(save) {
            var saveOrUpload = save ? qsTr("Save") : qsTr("Upload")
            mainWindow.showMessageDialog(qsTr("Unable to %1").arg(saveOrUpload), qsTr("Plan has incomplete items. Complete all items and %1 again.").arg(saveOrUpload))
        }

        function waitingOnTerrainDataMessage(save) {
            var saveOrUpload = save ? qsTr("Save") : qsTr("Upload")
            mainWindow.showMessageDialog(qsTr("Unable to %1").arg(saveOrUpload), qsTr("Plan is waiting on terrain data from server for correct altitude values."))
        }

        function checkReadyForSaveUpload(save) {
            if (readyForSaveState() == VisualMissionItem.NotReadyForSaveData) {
                waitingOnIncompleteDataMessage(save)
                return false
            } else if (readyForSaveState() == VisualMissionItem.NotReadyForSaveTerrain) {
                waitingOnTerrainDataMessage(save)
                return false
            }
            return true
        }

        function upload() {
            if (!checkReadyForSaveUpload(false /* save */)) {
                return
            }
            switch (_missionController.sendToVehiclePreCheck()) {
                case MissionController.SendToVehiclePreCheckStateOk:
                    sendToVehicle()
                    break
                case MissionController.SendToVehiclePreCheckStateActiveMission:
                    mainWindow.showMessageDialog(qsTr("Send To Vehicle"), qsTr("Current mission must be paused prior to uploading a new Plan"))
                    break
                case MissionController.SendToVehiclePreCheckStateFirwmareVehicleMismatch:
                    mainWindow.showMessageDialog(qsTr("Plan Upload"),
                                                 qsTr("This Plan was created for a different firmware or vehicle type than the firmware/vehicle type of vehicle you are uploading to. " +
                                                      "This can lead to errors or incorrect behavior. " +
                                                      "It is recommended to recreate the Plan for the correct firmware/vehicle type.\n\n" +
                                                      "Click 'Ok' to upload the Plan anyway."),
                                                 Dialog.Ok | Dialog.Cancel,
                                                 function() { _planMasterController.sendToVehicle() })
                    break
            }
        }

        function loadFromSelectedFile() {
            fileDialog.title =          qsTr("Select Plan File")
            fileDialog.planFiles =      true
            fileDialog.nameFilters =    _planMasterController.loadNameFilters
            fileDialog.openForLoad()
        }

        function saveToSelectedFile() {
            if (!checkReadyForSaveUpload(true /* save */)) {
                return
            }
            fileDialog.title =          qsTr("Save Plan")
            fileDialog.planFiles =      true
            fileDialog.nameFilters =    _planMasterController.saveNameFilters
            fileDialog.openForSave()
        }

        function fitViewportToItems() {
            mapFitFunctions.fitMapViewportToMissionItems()
        }

        function saveKmlToSelectedFile() {
            if (!checkReadyForSaveUpload(true /* save */)) {
                return
            }
            fileDialog.title =          qsTr("Save KML")
            fileDialog.planFiles =      false
            fileDialog.nameFilters =    ShapeFileHelper.fileDialogKMLFilters
            fileDialog.openForSave()
        }
    }

    Connections {
        target: _missionController

        function onNewItemsFromVehicle() {
            if (_visualItems && _visualItems.count !== 1) {
                mapFitFunctions.fitMapViewportToMissionItems()
            }
            _missionController.setCurrentPlanViewSeqNum(0, true)
        }
    }

    function insertSimpleItemAfterCurrent(coordinate) {
        var nextIndex = _missionController.currentPlanViewVIIndex + 1
        _missionController.insertSimpleMissionItem(coordinate, nextIndex, true /* makeCurrentItem */)
    }

    function insertROIAfterCurrent(coordinate) {
        var nextIndex = _missionController.currentPlanViewVIIndex + 1
        _missionController.insertROIMissionItem(coordinate, nextIndex, true /* makeCurrentItem */)
    }

    function insertCancelROIAfterCurrent() {
        var nextIndex = _missionController.currentPlanViewVIIndex + 1
        _missionController.insertCancelROIMissionItem(nextIndex, true /* makeCurrentItem */)
    }

    function insertComplexItemAfterCurrent(complexItemName) {
        var nextIndex = _missionController.currentPlanViewVIIndex + 1
        _missionController.insertComplexMissionItem(complexItemName, mapCenter(), nextIndex, true /* makeCurrentItem */)
    }

    function insertTakeoffItemAfterCurrent() {
        var nextIndex = _missionController.currentPlanViewVIIndex + 1
        _missionController.insertTakeoffItem(mapCenter(), nextIndex, true /* makeCurrentItem */)
    }

    function insertLandItemAfterCurrent() {
        var nextIndex = _missionController.currentPlanViewVIIndex + 1
        _missionController.insertLandItem(mapCenter(), nextIndex, true /* makeCurrentItem */)
    }


    function selectNextNotReady() {
        var foundCurrent = false
        for (var i=0; i<_missionController.visualItems.count; i++) {
            var vmi = _missionController.visualItems.get(i)
            if (vmi.readyForSaveState === VisualMissionItem.NotReadyForSaveData) {
                _missionController.setCurrentPlanViewSeqNum(vmi.sequenceNumber, true)
                break
            }
        }
    }

    QGCFileDialog {
        id:             fileDialog
        folder:         _appSettings ? _appSettings.missionSavePath : ""

        property bool planFiles: true    ///< true: working with plan files, false: working with kml file

        onAcceptedForSave: (file) => {
            if (planFiles) {
                _planMasterController.saveToFile(file)
            } else {
                _planMasterController.saveToKml(file)
            }
            close()
        }

        onAcceptedForLoad: (file) => {
            _planMasterController.loadFromFile(file)
            _planMasterController.fitViewportToItems()
            _missionController.setCurrentPlanViewSeqNum(0, true)
            close()
        }
    }

    // The map is the view; the toolbar rests on it. It used to be a solid bar with the map
    // starting underneath its bottom edge, which cut the one continuous surface this view has
    // into a strip and a box.
    PlanViewToolBar {
        id:                     planToolBar
        planMasterController:   _planMasterController
        rightInset:             rightPanel.visible ? _rightPanelWidth + _panelMargin : 0
        z:                      QGroundControl.zOrderWidgets + 1
    }

    Item {
        id:             panel
        anchors.fill:   parent

        FlightMap {
            id:                         editorMap
            anchors.fill:               parent
            mapName:                    "MissionEditor"
            allowGCSLocationCenter:     true
            allowVehicleLocationCenter: true
            planView:                   true

            zoomLevel:                  QGroundControl.flightMapZoom
            center:                     QGroundControl.flightMapPosition

            // This is the center rectangle of the map which is not obscured by tools
            // Reads the reservation, not the profile's y. Deriving the map's usable centre from
            // a child's position meant the fit-to-view geometry changed as the profile
            // animated, and any change to the profile's layout silently moved it.
            property rect centerViewport:   Qt.rect(_leftToolWidth + _margin,
                                                    _topReserved + _margin,
                                                    editorMap.width - _leftToolWidth - _rightToolWidth - (_margin * 2),
                                                    editorMap.height - _topReserved - _terrainReserved - (_margin * 2))

            property real _terrainReserved: _root._terrainProfileOpen ? _root._terrainProfileHeight : 0
            property real _topReserved:     planToolBar.height

            property real _leftToolWidth:       dock.x + dock.width
            property real _rightToolWidth:      rightPanel.width + _root._panelMargin
            property real _nonInteractiveOpacity:  0.5

            // Initial map position duplicates Fly view position
            Component.onCompleted: editorMap.center = QGroundControl.flightMapPosition

            QGCMapPalette { id: mapPal; lightColors: editorMap.isSatelliteMap }

            onZoomLevelChanged: {
                QGroundControl.flightMapZoom = editorMap.zoomLevel
                OverlayBackdrop.refresh()
            }
            onCenterChanged: {
                QGroundControl.flightMapPosition = editorMap.center
                OverlayBackdrop.refresh()
            }

            onMapClicked: (mouse) => {
                // Take focus to close any previous editing
                editorMap.focus = true
                if (!mainWindow.allowViewSwitch()) {
                    return
                }
                var coordinate = editorMap.toCoordinate(Qt.point(mouse.x, mouse.y), false /* clipToViewPort */)
                coordinate.latitude = coordinate.latitude.toFixed(_decimalPlaces)
                coordinate.longitude = coordinate.longitude.toFixed(_decimalPlaces)
                coordinate.altitude = coordinate.altitude.toFixed(_decimalPlaces)
				if(_utmspEnabled){
                	QGroundControl.utmspManager.utmspVehicle.updateLastCoordinates(coordinate.latitude, coordinate.longitude)
                }
                
                switch (_editingLayer) {
                case _layerMission:
                case _layerUTMSP:
                    if (_addWaypointMode) {
                        insertSimpleItemAfterCurrent(coordinate)
                    } else if (_addROIOnClick) {
                        insertROIAfterCurrent(coordinate)
                        _addROIOnClick = false
                    }
                    break
                case _layerRallyPoints:
                    if (_rallyPointController.supported && _addWaypointMode) {
                        _rallyPointController.addPoint(coordinate)
                    }
                    break
                }
            }

            // Add the mission item visuals to the map
            Repeater {
                model: _missionController.visualItems
                delegate: MissionItemMapVisual {
                    map:         editorMap
                    opacity:     _editingLayer == _layerMission || _editingLayer == _layerUTMSP ? 1 : editorMap._nonInteractiveOpacity
                    interactive: _editingLayer == _layerMission || _editingLayer == _layerUTMSP
                    vehicle:     _planMasterController.controllerVehicle
                    onClicked:   (sequenceNumber) => { _missionController.setCurrentPlanViewSeqNum(sequenceNumber, false) }
                }
            }

            // Add lines between waypoints
            MissionLineView {
                showSpecialVisual:  _missionController.isROIBeginCurrentItem
                model:              _missionController.simpleFlightPathSegments
                opacity:            _editingLayer == _layerMission ||  _editingLayer == _layerUTMSP  ? 1 : editorMap._nonInteractiveOpacity
            }

            // Direction arrows in waypoint lines
            MapItemView {
                model: _editingLayer == _layerMission ||_editingLayer == _layerUTMSP ? _missionController.directionArrows : undefined

                delegate: MapLineArrow {
                    fromCoord:      object ? object.coordinate1 : undefined
                    toCoord:        object ? object.coordinate2 : undefined
                    arrowPosition:  3
                    z:              QGroundControl.zOrderWaypointLines + 1
                }
            }

            // Incomplete segment lines
            MapItemView {
                model: _missionController.incompleteComplexItemLines

                delegate: MapPolyline {
                    path:       [ object.coordinate1, object.coordinate2 ]
                    line.width: 1
                    line.color: "red"
                    z:          QGroundControl.zOrderWaypointLines
                    opacity:    _editingLayer == _layerMission ? 1 : editorMap._nonInteractiveOpacity
                }
            }

            // UI for splitting the current segment
            MapQuickItem {
                id:             splitSegmentItem
                anchorPoint.x:  sourceItem.width / 2
                anchorPoint.y:  sourceItem.height / 2
                z:              QGroundControl.zOrderWaypointLines + 1
                visible:        _editingLayer == _layerMission ||  _editingLayer == _layerUTMSP

                sourceItem: SplitIndicator {
                    onClicked:  _missionController.insertSimpleMissionItem(splitSegmentItem.coordinate,
                                                                           _missionController.currentPlanViewVIIndex,
                                                                           true /* makeCurrentItem */)
                }

                function _updateSplitCoord() {
                    if (_missionController.splitSegment) {
                        var distance = _missionController.splitSegment.coordinate1.distanceTo(_missionController.splitSegment.coordinate2)
                        var azimuth = _missionController.splitSegment.coordinate1.azimuthTo(_missionController.splitSegment.coordinate2)
                        splitSegmentItem.coordinate = _missionController.splitSegment.coordinate1.atDistanceAndAzimuth(distance / 2, azimuth)
                    } else {
                        coordinate = QtPositioning.coordinate()
                    }
                }

                Connections {
                    target:                 _missionController
                    function onSplitSegmentChanged()  { splitSegmentItem._updateSplitCoord() }
                }

                Connections {
                    target:                 _missionController.splitSegment
                    function onCoordinate1Changed()   { splitSegmentItem._updateSplitCoord() }
                    function onCoordinate2Changed()   { splitSegmentItem._updateSplitCoord() }
                }
            }

            // Add the vehicles to the map
            MapItemView {
                model: QGroundControl.multiVehicleManager.vehicles
                delegate: VehicleMapItem {
                    vehicle:        object
                    coordinate:     object.coordinate
                    map:            editorMap
                    size:           ScreenTools.defaultFontPixelHeight * 3
                    z:              QGroundControl.zOrderMapItems - 1
                }
            }

            GeoFenceMapVisuals {
                map:                    editorMap
                myGeoFenceController:   _geoFenceController
                interactive:            _editingLayer == _layerGeoFence
                homePosition:           _missionController.plannedHomePosition
                planView:               true
                opacity:                _editingLayer != _layerGeoFence ? editorMap._nonInteractiveOpacity : 1
            }

            RallyPointMapVisuals {
                map:                    editorMap
                myRallyPointController: _rallyPointController
                interactive:            _editingLayer == _layerRallyPoints
                planView:               true
                opacity:                _editingLayer != _layerRallyPoints ? editorMap._nonInteractiveOpacity : 1
            }

            UTMSPMapVisuals {
                id: utmspvisual
                enabled:                _utmspEnabled
                map:                    editorMap
                currentMissionItems:    _visualItems
                myGeoFenceController:   _geoFenceController
                interactive:            _editingLayer == _layerUTMSP
                homePosition:           _missionController.plannedHomePosition
                planView:               true
                opacity:                _editingLayer != _layerUTMSP ? editorMap._nonInteractiveOpacity : 1
                resetCheck:             _resetGeofencePolygon
            }

            Connections {
                target: utmspEditor
                function onResetGeofencePolygonTriggered() {
                    resetTimer.start()
                }
            }
            Timer {
                id: resetTimer
                interval: 2500
                running: false
                repeat: false
                onTriggered: {
                    _resetGeofencePolygon = true
                }
            }
        }

        //-----------------------------------------------------------
        // What the glass in this view refracts. Every overlay surface here used to sample the
        // fly view's backdrop, because that was the only one registered - so the panels frosted
        // a blurred snapshot of a different map, offset from the one they were sitting on. That
        // is what made the material read as a picture underneath rather than as glass.
        ShaderEffectSource {
            id:             planBackdropCapture
            anchors.fill:   editorMap
            sourceItem:     editorMap
            live:           false
            visible:        false
            textureSize:    Qt.size(Math.max(1, Math.round(editorMap.width  / _downscale)),
                                    Math.max(1, Math.round(editorMap.height / _downscale)))

            readonly property int _downscale: 4
        }

        MultiEffect {
            id:            planFrostedBackdrop
            anchors.fill:  editorMap
            source:        planBackdropCapture
            blurEnabled:   true
            blur:          1.0
            blurMax:       32
            saturation:    0.25
            visible:       false
            layer.enabled: true

            Component.onCompleted:   OverlayBackdrop.addScope(_root, planFrostedBackdrop)
            Component.onDestruction:  OverlayBackdrop.removeScope(_root)

            Connections {
                target: OverlayBackdrop
                function onRefreshed() { planBackdropCapture.scheduleUpdate() }
            }
        }

        //-----------------------------------------------------------
        // Left dock: file · edit · view. The strip used to be a column of labelled tiles pinned
        // to the top-left corner, sized by its own maxHeight against the window. A dock is one
        // object centred on the edge it belongs to, and the hairlines say which buttons are
        // about the file, which edit the plan, and which only move the camera.
        Rectangle {
            id:                     dock
            anchors.left:           parent.left
            anchors.leftMargin:     _panelMargin
            anchors.verticalCenter: parent.verticalCenter
            width:                  dockColumn.width + _dockPadding * 2
            height:                 dockColumn.height + _dockPadding * 2
            radius:                 width / 2
            color:                  "transparent"
            z:                      QGroundControl.zOrderWidgets
            layer.enabled:          true
            layer.effect:           OverlayShadowEffect { elevated: true }

            readonly property real _dockPadding: ScreenTools.defaultFontPixelHeight * 0.35

            OverlayGlass {
                id:            dockGlass
                anchors.fill:  parent
                radius:        parent.radius
                lightMaterial: false
                tint:          Qt.alpha(_qgcPal.window, 0.72)
                minTint:       0.7
                maxTint:       0.92
            }

            component DockButton: Rectangle {
                id:      dockButton
                width:   Math.max(ScreenTools.minTouchPixels, ScreenTools.defaultFontPixelHeight * 2.2)
                height:  width
                radius:  width / 2
                color:   !dockButton.enabled       ? "transparent"
                       : dockButton.checked        ? Qt.alpha(_qgcPal.primaryButton, 0.34)
                       : dockMouseArea.containsMouse ? Qt.alpha(_qgcPal.text, 0.08)
                                                     : "transparent"

                property string icon
                property bool   checked: false

                signal clicked()

                Behavior on color { ColorAnimation { duration: 120 } }

                QGCColoredImage {
                    anchors.centerIn:   parent
                    source:             dockButton.icon
                    height:             parent.height * 0.5
                    width:              height
                    sourceSize.height:  height
                    fillMode:           Image.PreserveAspectFit
                    mipmap:             true
                    smooth:             true
                    color:              !dockButton.enabled ? Qt.alpha(_qgcPal.text, 0.35)
                                      : dockButton.checked  ? _qgcPal.primaryButton
                                                            : _qgcPal.text
                }

                QGCMouseArea {
                    id:             dockMouseArea
                    anchors.fill:   parent
                    hoverEnabled:   !ScreenTools.isMobile
                    enabled:        dockButton.enabled
                    onClicked:      dockButton.clicked()
                }
            }

            component DockSeparator: Rectangle {
                anchors.horizontalCenter: parent ? parent.horizontalCenter : undefined
                width:  ScreenTools.defaultFontPixelHeight * 1.4
                height: 1
                color:  Qt.alpha(_qgcPal.text, 0.12)
            }

            Column {
                id:                 dockColumn
                anchors.centerIn:   parent
                spacing:            ScreenTools.defaultFontPixelHeight * 0.22

                DockButton {
                    id:         fileButton
                    objectName: "planDockFile"
                    icon:    _planMasterController.dirty ? "/qmlimages/MapSyncChanged.svg" : "/qmlimages/MapSync.svg"
                    enabled: !_planMasterController.syncInProgress
                    onClicked: {
                        allAddClickBoolsOff()
                        fileMenu.openFrom(fileButton)
                    }
                }

                DockSeparator {}

                DockButton {
                    id:         addWaypointButton
                    objectName: "planDockAddWaypoint"
                    icon:    "/qmlimages/MapAddMission.svg"
                    checked: _addWaypointMode
                    enabled: _editingLayer === _layerRallyPoints ? _rallyPointController.supported
                                                                 : _missionController.flyThroughCommandsAllowed
                    visible: _editingLayer === _layerMission || _editingLayer === _layerRallyPoints || _editingLayer === _layerUTMSP
                    onClicked: {
                        _addROIOnClick   = false
                        _addWaypointMode = !_addWaypointMode
                    }
                }

                DockButton {
                    id:         addButton
                    objectName: "planDockAdd"
                    icon:    "/qmlimages/MapDrawShape.svg"
                    checked: _addROIOnClick
                    visible: _editingLayer === _layerMission || _editingLayer === _layerUTMSP
                    onClicked: {
                        _addWaypointMode = false
                        addMenu.openFrom(addButton)
                    }
                }

                DockSeparator {}

                DockButton {
                    id:         centerButton
                    objectName: "planDockCenter"
                    icon:      "/qmlimages/MapCenter.svg"
                    onClicked: centerMenu.openFrom(centerButton)
                }

                DockButton {
                    id:         mapTypeButton
                    objectName: "planDockMapType"
                    icon:      "/qmlimages/MapType.svg"
                    onClicked: mapTypeMenu.openFrom(mapTypeButton)
                }
            }
        }

        //-----------------------------------------------------------
        // Dock menus. These were drop panels of stacked QGCButtons under collapsible section
        // headers - a form, opened to pick one thing. A menu is a list of choices with the
        // destructive one set apart at the end.
        OverlayPopover {
            id: fileMenu

            OverlayMenuItem {
                text:    qsTr("Open…")
                enabled: !_planMasterController.syncInProgress
                onClicked: {
                    fileMenu.close()
                    if (_planMasterController.dirty) {
                        showLoadFromFileOverwritePrompt(qsTr("Plan overwrite"))
                    } else {
                        _planMasterController.loadFromSelectedFile()
                    }
                }
            }

            OverlayMenuItem {
                text:    qsTr("Save")
                enabled: !_planMasterController.syncInProgress && _planMasterController.containsItems
                onClicked: {
                    fileMenu.close()
                    if (_planMasterController.currentPlanFile !== "") {
                        _planMasterController.saveToCurrent()
                    } else {
                        _planMasterController.saveToSelectedFile()
                    }
                }
            }

            OverlayMenuItem {
                text:    qsTr("Save As…")
                enabled: !_planMasterController.syncInProgress && _planMasterController.containsItems
                onClicked: {
                    fileMenu.close()
                    _planMasterController.saveToSelectedFile()
                }
            }

            OverlayMenuItem {
                text:    qsTr("Export KML…")
                enabled: !_planMasterController.syncInProgress && _visualItems.count > 1
                onClicked: {
                    fileMenu.close()
                    _planMasterController.saveKmlToSelectedFile()
                }
            }

            OverlayMenuSeparator {}

            OverlayMenuItem {
                text:    qsTr("New Plan…")
                onClicked: {
                    fileMenu.close()
                    planCreatorMenu.openFrom(fileButton)
                }
            }

            OverlayMenuItem {
                text:    qsTr("Download from Vehicle")
                enabled: !_planMasterController.offline && !_planMasterController.syncInProgress
                visible: !QGroundControl.corePlugin.options.disableVehicleConnection
                onClicked: {
                    fileMenu.close()
                    downloadClicked(qsTr("Plan overwrite"))
                }
            }

            OverlayMenuSeparator { visible: clearMissionItem.visible }

            OverlayMenuItem {
                id:         clearMissionItem
                text:       qsTr("Clear Mission")
                textColor:  _qgcPal.colorRed
                enabled:    !_planMasterController.offline && !_planMasterController.syncInProgress
                visible:    !QGroundControl.corePlugin.options.disableVehicleConnection
                onClicked: {
                    fileMenu.close()
                    clearButtonClicked()
                }
            }
        }

        OverlayPopover {
            id: planCreatorMenu

            Repeater {
                model: _planMasterController.planCreators

                OverlayMenuItem {
                    text: object.name
                    onClicked: {
                        planCreatorMenu.close()
                        const centerPoint = Qt.point(editorMap.centerViewport.left + (editorMap.centerViewport.width / 2),
                                                     editorMap.centerViewport.top + (editorMap.centerViewport.height / 2))
                        const coord = editorMap.toCoordinate(centerPoint, false /* clipToViewPort */)
                        if (_planMasterController.containsItems) {
                            createPlanRemoveAllPromptDialog.createObject(mainWindow, { mapCenter: coord, planCreator: object }).open()
                        } else {
                            object.createPlan(coord)
                        }
                    }
                }
            }
        }

        OverlayPopover {
            id: addMenu

            OverlayMenuItem {
                objectName: "planAddTakeoff"
                text:    qsTr("Takeoff")
                enabled: _missionController.isInsertTakeoffValid
                visible: !_planMasterController.controllerVehicle.rover
                onClicked: {
                    addMenu.close()
                    allAddClickBoolsOff()
                    insertTakeoffItemAfterCurrent()
                    _triggerSubmit = true
                }
            }

            OverlayMenuItem {
                text:    _missionController.isROIActive ? qsTr("Cancel ROI") : qsTr("Region of Interest")
                enabled: !_missionController.onlyInsertTakeoffValid
                visible: _planMasterController.controllerVehicle.roiModeSupported
                onClicked: {
                    addMenu.close()
                    if (_missionController.isROIActive) {
                        allAddClickBoolsOff()
                        insertCancelROIAfterCurrent()
                    } else {
                        _addWaypointMode = false
                        _addROIOnClick   = true
                    }
                }
            }

            OverlayMenuItem {
                text:    _planMasterController.controllerVehicle.multiRotor
                             ? qsTr("Return")
                             : _missionController.isInsertLandValid && _missionController.hasLandItem
                               ? qsTr("Alt Land")
                               : qsTr("Land")
                enabled: _missionController.isInsertLandValid
                onClicked: {
                    addMenu.close()
                    allAddClickBoolsOff()
                    insertLandItemAfterCurrent()
                }
            }

            OverlayMenuSeparator {}

            Repeater {
                model: _missionController.complexMissionItemNames

                OverlayMenuItem {
                    objectName: "planAddPattern"
                    text:    modelData
                    enabled: _missionController.flyThroughCommandsAllowed
                    onClicked: {
                        addMenu.close()
                        allAddClickBoolsOff()
                        insertComplexItemAfterCurrent(modelData)
                    }
                }
            }
        }

        OverlayPopover {
            id: centerMenu

            OverlayMenuItem {
                text: qsTr("Mission")
                onClicked: {
                    centerMenu.close()
                    mapFitFunctions.fitMapViewportToMissionItems()
                }
            }

            OverlayMenuItem {
                text: qsTr("All Items")
                onClicked: {
                    centerMenu.close()
                    mapFitFunctions.fitMapViewportToAllItems()
                }
            }

            OverlayMenuItem {
                text:    qsTr("Launch")
                enabled: _missionController.plannedHomePosition.isValid
                onClicked: {
                    centerMenu.close()
                    editorMap.center = _missionController.plannedHomePosition
                }
            }

            OverlayMenuItem {
                text:    qsTr("Vehicle")
                enabled: globals.activeVehicle && globals.activeVehicle.coordinate.isValid
                onClicked: {
                    centerMenu.close()
                    editorMap.center = globals.activeVehicle.coordinate
                }
            }
        }

        // The map's own look, one tap from the map - not three levels into application settings.
        OverlayPopover {
            id: mapTypeMenu

            readonly property var _mapTypeFact:     QGroundControl.settingsManager.flightMapSettings.mapType
            readonly property var _mapProviderFact: QGroundControl.settingsManager.flightMapSettings.mapProvider

            Repeater {
                model: QGroundControl.mapEngineManager.mapTypeList(mapTypeMenu._mapProviderFact.rawValue)

                OverlayMenuItem {
                    objectName: "planMapType"
                    text:       modelData
                    checkable:  true
                    checked:    mapTypeMenu._mapTypeFact.rawValue === modelData
                    onClicked: {
                        mapTypeMenu.close()
                        mapTypeMenu._mapTypeFact.rawValue = modelData
                    }
                }
            }
        }

        //-----------------------------------------------------------
        // Right inspector.
        //
        // One floating panel. This was a full-height slab butted against the window edge with a
        // tab bar for a header - the map stopped where it began. Inset on every side, the map
        // runs behind it and the panel reads as resting on the plan rather than framing it.
        Item {
            id:                     rightPanel
            width:                  _rightPanelWidth
            anchors.top:            parent.top
            anchors.topMargin:      planToolBar.height + _panelMargin
            anchors.bottom:         parent.bottom
            anchors.bottomMargin:   _panelMargin
            anchors.right:          parent.right
            anchors.rightMargin:    _panelMargin
            z:                      QGroundControl.zOrderWidgets
            visible:                _editingLayer !== 0

            readonly property real _padding: ScreenTools.defaultFontPixelHeight * 0.7

            Rectangle {
                anchors.fill:   parent
                radius:         _panelRadius
                color:          "transparent"
                layer.enabled:  true
                layer.effect:   OverlayShadowEffect { elevated: true }

                // A capsule can be thin because it covers a few words; a full-height panel of
                // forms cannot - over bright ground the map reads straight through the text.
                // More tint, same refraction and rim.
                OverlayGlass {
                    id:            panelGlass
                    anchors.fill:  parent
                    radius:        _panelRadius
                    lightMaterial: false
                    // The shared glass tint is 45% opaque, which is right for a pill holding two
                    // words and far too thin under a panel of forms - over bright imagery the
                    // map reads straight through the labels. Same material, more of it.
                    tint:         Qt.alpha(_qgcPal.window, 0.86)
                    minTint:      0.82
                    maxTint:      0.96
                }
            }

            DeadMouseArea {
                anchors.fill:   parent
            }

            // One control built from the layer list, so a layer is added in one place. Mission,
            // Fence and Rally are modes over the same canvas rather than separate pages, which
            // is a segmented control, not a tab bar.
            OverlaySegmentedControl {
                id:                  layerSelector
                objectName:          "planLayerSelector"
                anchors.top:         parent.top
                anchors.left:        parent.left
                anchors.right:       parent.right
                anchors.margins:     rightPanel._padding
                visible:             QGroundControl.corePlugin.options.enablePlanViewSelector
                currentIndex:        _layerIndex
                onActivated: (index) => { _layerIndex = index }

                segments: _activeLayers.map(layer => ({
                    text:    layer === _layerMission     ? qsTr("Mission")
                           : layer === _layerGeoFence    ? qsTr("Fence")
                           : layer === _layerRallyPoints ? qsTr("Rally")
                                                         : qsTr("UTM-SP"),
                    enabled: layer === _layerGeoFence    ? _geoFenceController.supported
                           : layer === _layerRallyPoints ? _rallyPointController.supported
                                                         : true
                }))
            }

            Item {
                id:                   rightPanelBody
                anchors.top:          layerSelector.visible ? layerSelector.bottom : parent.top
                anchors.topMargin:    rightPanel._padding
                anchors.left:         parent.left
                anchors.right:        parent.right
                anchors.bottom:       parent.bottom
                anchors.leftMargin:   rightPanel._padding
                anchors.rightMargin:  rightPanel._padding
                anchors.bottomMargin: rightPanel._padding

                //-------------------------------------------------------
                // Mission Start above, the items below. Mission Start used to be the first row of
                // the item list, so selecting it drew a filled block the height of a form and the
                // settings inside it needed a box of their own to look like groups - a card in a
                // card in a card. It is what it always was: a section, not an item.
                QGCFlickable {
                    id:             missionItemEditor
                    anchors.fill:   parent
                    contentHeight:  missionColumn.height
                    clip:           true
                    visible:        _editingLayer === _layerMission && !planControlColapsed && !_showItemDetail

                    readonly property var _missionSettingsItem: (_visualItems && _visualItems.count > 0) ? _visualItems.get(0) : null

                    Column {
                        id:             missionColumn
                        anchors.left:   parent.left
                        anchors.right:  parent.right
                        spacing:        ScreenTools.defaultFontPixelHeight * 0.5

                        Loader {
                            id:             missionSettingsLoader
                            width:          parent.width
                            // No explicit height: the loader takes its implicit height from the
                            // editor it holds, so the column below it stays put as the editor
                            // grows and shrinks with the vehicle's options.
                            active:         missionItemEditor._missionSettingsItem !== null
                            source:         missionItemEditor._missionSettingsItem ? missionItemEditor._missionSettingsItem.editorQml : ""

                            property var  masterController:     _planMasterController
                            property var  missionItem:          missionItemEditor._missionSettingsItem
                            property var  map:                  editorMap
                            property real availableWidth:       missionSettingsLoader.width
                            property var  editorRoot:           missionSettingsLoader
                            property bool _noMissionItemsAdded: _visualItems.count === 1
                        }

                        QGCLabel {
                            text:           qsTr("ITEMS")
                            font.pointSize: ScreenTools.smallFontPointSize
                            font.bold:      true
                            color:          Qt.alpha(_qgcPal.text, 0.5)
                        }

                        PlanGroupCard {
                            width: parent.width

                            Repeater {
                                model: _visualItems

                                MissionItemEditor {
                                    map:              editorMap
                                    masterController: _planMasterController
                                    missionItem:      object
                                    width:            missionColumn.width
                                    readOnly:         false
                                    // Sequence 0 is the mission settings item, shown above as its
                                    // own section rather than as a row of this list.
                                    visible:          index > 0
                                    onClicked: (sequenceNumber) => { _missionController.setCurrentPlanViewSeqNum(object.sequenceNumber, false) }
                                    onRemove: {
                                        var removeVIIndex = index
                                        _missionController.removeVisualItem(removeVIIndex)
                                        if (removeVIIndex >= _missionController.visualItems.count) {
                                            removeVIIndex--
                                        }
                                    }
                                    onSelectNextNotReadyItem:   selectNextNotReady()
                                }
                            }

                            PlanGroupRow {
                                text:        qsTr("＋  Add waypoint")
                                textColor:   _qgcPal.primaryButton
                                interactive: true
                                enabled:     _missionController.flyThroughCommandsAllowed
                                onClicked:   insertSimpleItemAfterCurrent(mapCenter())
                            }
                        }
                    }
                }

                //-------------------------------------------------------
                // Complex item detail. A survey or scan carries a page of its own settings, which
                // cannot live inline under a list row - so it pushes a page, with the list as its
                // root and one way back.
                Item {
                    id:             itemDetail
                    anchors.fill:   parent
                    visible:        _editingLayer === _layerMission && _showItemDetail

                    Item {
                        id:             detailHeader
                        anchors.top:    parent.top
                        anchors.left:   parent.left
                        anchors.right:  parent.right
                        height:         ScreenTools.defaultFontPixelHeight * 2

                        QGCLabel {
                            id:                     backLabel
                            anchors.left:           parent.left
                            anchors.verticalCenter: parent.verticalCenter
                            text:                   qsTr("‹  Items")
                            color:                  _qgcPal.primaryButton
                        }

                        QGCMouseArea {
                            anchors.fill:       backLabel
                            anchors.margins:    -ScreenTools.defaultFontPixelWidth
                            onClicked:          _missionController.setCurrentPlanViewSeqNum(0, true)
                        }

                        QGCLabel {
                            anchors.centerIn:   parent
                            text:               _currentPlanItem ? _currentPlanItem.commandName : ""
                            font.bold:          true
                        }
                    }

                    QGCFlickable {
                        anchors.top:            detailHeader.bottom
                        anchors.topMargin:      ScreenTools.defaultFontPixelHeight * 0.4
                        anchors.left:           parent.left
                        anchors.right:          parent.right
                        anchors.bottom:         parent.bottom
                        contentHeight:          detailColumn.height
                        flickableDirection:     Flickable.VerticalFlick
                        clip:                   true

                        Column {
                            id:         detailColumn
                            width:      parent.width
                            spacing:    ScreenTools.defaultFontPixelHeight * 0.7

                            Rectangle {
                                width:   parent.width
                                height:  detailLoader.height + ScreenTools.defaultFontPixelHeight * 0.8
                                radius:  ScreenTools.defaultFontPixelHeight * 0.9
                                color:   Qt.alpha(_qgcPal.text, 0.055)

                                Loader {
                                    id:                 detailLoader
                                    anchors.top:        parent.top
                                    anchors.topMargin:  ScreenTools.defaultFontPixelHeight * 0.4
                                    anchors.left:       parent.left
                                    anchors.leftMargin: ScreenTools.defaultFontPixelWidth * 1.5
                                    width:              parent.width - ScreenTools.defaultFontPixelWidth * 3
                                    source:             _showItemDetail ? _currentPlanItem.editorQml : ""
                                    asynchronous:       true

                                    property var  masterController: _planMasterController
                                    property real availableWidth:   detailLoader.width
                                    property var  editorRoot:       detailLoader
                                    property var  missionItem:      _currentPlanItem
                                    // Complex editors and the camera sections they load read
                                    // these from the scope their host used to provide.
                                    property real _editFieldWidth:  Math.min(detailLoader.width - ScreenTools.defaultFontPixelWidth * 2,
                                                                             ScreenTools.defaultFontPixelWidth * 14)
                                    property real _margin:          ScreenTools.defaultFontPixelWidth / 2
                                    property var  _missionController: _root._missionController
                                }
                            }

                            PlanGroupCard {
                                width: parent.width

                                PlanGroupRow {
                                    text:        qsTr("Delete %1").arg(_currentPlanItem ? _currentPlanItem.commandName : "")
                                    textColor:   _qgcPal.colorRed
                                    interactive: true
                                    onClicked: {
                                        const seq = _currentPlanItem.sequenceNumber
                                        _missionController.setCurrentPlanViewSeqNum(0, true)
                                        _missionController.removeVisualItem(seq)
                                    }
                                }
                            }
                        }
                    }
                }

                // GeoFence Editor
                GeoFenceEditor {
                    anchors.top:            parent.top
                    anchors.bottom:         parent.bottom
                    anchors.left:           parent.left
                    anchors.right:          parent.right
                    myGeoFenceController:   _geoFenceController
                    flightMap:              editorMap
                    visible:                _editingLayer == _layerGeoFence
                }

                // Rally points, as one grouped list. There used to be a description card and,
                // below it, an editor for whichever point happened to be current - so the set of
                // rally points was only ever visible on the map, never in the panel.
                QGCFlickable {
                    anchors.fill:       parent
                    contentHeight:      rallyColumn.height
                    clip:               true
                    visible:            _editingLayer === _layerRallyPoints

                    Column {
                        id:             rallyColumn
                        anchors.left:   parent.left
                        anchors.right:  parent.right
                        spacing:        ScreenTools.defaultFontPixelHeight * 0.5

                        PlanGroupCard {
                            width:      parent.width
                            visible:    !_rallyPointController.supported || _rallyPointController.points.count === 0

                            Column {
                                width:         parent.width
                                topPadding:    ScreenTools.defaultFontPixelHeight
                                bottomPadding: ScreenTools.defaultFontPixelHeight
                                spacing:       ScreenTools.defaultFontPixelHeight * 0.5

                                QGCLabel {
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    font.bold:                true
                                    text:                     _rallyPointController.supported ? qsTr("No rally points")
                                                                                              : qsTr("Not supported")
                                }

                                QGCLabel {
                                    width:                    parent.width - ScreenTools.defaultFontPixelWidth * 4
                                    anchors.horizontalCenter: parent.horizontalCenter
                                    horizontalAlignment:      Text.AlignHCenter
                                    wrapMode:                 Text.WordWrap
                                    font.pointSize:           ScreenTools.smallFontPointSize
                                    color:                    Qt.alpha(_qgcPal.text, 0.5)
                                    text:                     _rallyPointController.supported
                                                                  ? qsTr("Alternate landing points for Return to Launch. Turn on the waypoint tool and tap the map to place one.")
                                                                  : qsTr("This vehicle does not support Rally Points.")
                                }
                            }
                        }

                        QGCLabel {
                            text:           qsTr("RALLY POINTS")
                            font.pointSize: ScreenTools.smallFontPointSize
                            font.bold:      true
                            color:          Qt.alpha(_qgcPal.text, 0.5)
                            visible:        _rallyPointController.points.count > 0
                        }

                        PlanGroupCard {
                            width:      parent.width
                            visible:    _rallyPointController.points.count > 0

                            Repeater {
                                model: _rallyPointController.points

                                Column {
                                    width: rallyColumn.width

                                    readonly property bool _isCurrent: object === _rallyPointController.currentRallyPoint

                                    PlanGroupRow {
                                        id:          rallyRow
                                        text:        qsTr("Rally %1").arg(index + 1)
                                        description: object.coordinate.latitude.toFixed(6) + ", " + object.coordinate.longitude.toFixed(6)
                                        interactive: true
                                        color:       parent._isCurrent ? Qt.alpha(_qgcPal.primaryButton, 0.16) : "transparent"
                                        onClicked:   _rallyPointController.currentRallyPoint = object

                                        Rectangle {
                                            anchors.verticalCenter: parent.verticalCenter
                                            width:                  ScreenTools.defaultFontPixelHeight * 1.5
                                            height:                 width
                                            radius:                 width * 0.3
                                            color:                  _qgcPal.colorGreen

                                            QGCLabel {
                                                anchors.centerIn:   parent
                                                text:               index + 1
                                                color:              "white"
                                                font.bold:          true
                                                font.pointSize:     ScreenTools.smallFontPointSize
                                            }
                                        }
                                    }

                                    Column {
                                        width:          parent.width
                                        visible:        parent._isCurrent
                                        leftPadding:    ScreenTools.defaultFontPixelWidth * 1.5
                                        rightPadding:   ScreenTools.defaultFontPixelWidth * 1.5
                                        bottomPadding:  ScreenTools.defaultFontPixelHeight * 0.5
                                        spacing:        ScreenTools.defaultFontPixelHeight * 0.3

                                        Repeater {
                                            model: object.textFieldFacts

                                            Item {
                                                width:  rallyColumn.width - ScreenTools.defaultFontPixelWidth * 3
                                                height: rallyFactField.height

                                                QGCLabel {
                                                    anchors.left:           parent.left
                                                    anchors.verticalCenter: parent.verticalCenter
                                                    text:                   modelData.name
                                                }

                                                FactTextField {
                                                    id:                     rallyFactField
                                                    anchors.right:          parent.right
                                                    width:                  Math.min(parent.width * 0.5, ScreenTools.defaultFontPixelWidth * 12)
                                                    showUnits:              true
                                                    fact:                   modelData
                                                }
                                            }
                                        }

                                        PlanGroupRow {
                                            width:       rallyColumn.width - ScreenTools.defaultFontPixelWidth * 3
                                            text:        qsTr("Delete Rally Point")
                                            textColor:   _qgcPal.colorRed
                                            interactive: true
                                            onClicked:   _rallyPointController.removePoint(object)
                                        }
                                    }
                                }
                            }

                            PlanGroupRow {
                                text:        qsTr("＋  Add rally point")
                                textColor:   _qgcPal.primaryButton
                                interactive: true
                                onClicked:   _rallyPointController.addPoint(editorMap.center)
                            }
                        }

                        PlanGroupCard {
                            width:      parent.width
                            visible:    _rallyPointController.supported && _rallyPointController.points.count === 0

                            PlanGroupRow {
                                text:        qsTr("＋  Add rally point")
                                textColor:   _qgcPal.primaryButton
                                interactive: true
                                onClicked:   _rallyPointController.addPoint(editorMap.center)
                            }
                        }
                    }
                }
                UTMSPAdapterEditor{
                    id: utmspEditor
                    enabled:                 _utmspEnabled
                    anchors.top:             parent.top
                    anchors.bottom:          parent.bottom
                    anchors.left:            parent.left
                    anchors.right:           parent.right
                    currentMissionItems:     _visualItems
                    myGeoFenceController:    _geoFenceController
                    flightMap:               editorMap
                    visible:                 _editingLayer == _layerUTMSP
                    triggerSubmitButton:     _triggerSubmit
                    resetRegisterFlightPlan: _resetRegisterFlightPlan
                }
            }
        }

        QGCLabel {
            // Elevation provider notice on top of terrain plot
            readonly property string _licenseString: QGroundControl.elevationProviderNotice

            id:                         licenseLabel
            visible:                    terrainSheet.visible && _licenseString !== ""
            anchors.bottom:             terrainSheet.top
            anchors.horizontalCenter:   terrainSheet.horizontalCenter
            anchors.bottomMargin:       ScreenTools.defaultFontPixelWidth * 0.5
            font.pointSize:             ScreenTools.smallFontPointSize
            text:                       qsTr("Powered by %1").arg(_licenseString)
        }

        //-----------------------------------------------------------
        // Terrain profile, as a sheet. It used to run edge to edge along the bottom of the
        // window and under the dock, so the one surface the plan lives on was cut off at the
        // knees. It sits clear of the dock and the inspector, and the grabber both says it can
        // be dismissed and does the dismissing.
        Item {
            id:                     terrainSheet
            anchors.left:           dock.right
            anchors.leftMargin:     _panelMargin
            anchors.right:          rightPanel.left
            anchors.rightMargin:    _panelMargin
            anchors.bottom:         parent.bottom
            anchors.bottomMargin:   _panelMargin
            height:                 _root._terrainProfileOpen ? _root._terrainProfileHeight : 0
            visible:                height > 0
            z:                      QGroundControl.zOrderWidgets
            clip:                   true

            Behavior on height { NumberAnimation { duration: 220; easing.type: Easing.OutCubic } }

            readonly property real _grabberHeight: ScreenTools.defaultFontPixelHeight

            Rectangle {
                anchors.fill:   parent
                radius:         _panelRadius
                color:          "transparent"
                layer.enabled:  true
                layer.effect:   OverlayShadowEffect { elevated: true }

                OverlayGlass {
                    anchors.fill:  parent
                    radius:        _panelRadius
                    lightMaterial: false
                    // The shared glass tint is 45% opaque, which is right for a pill holding two
                    // words and far too thin under a panel of forms - over bright imagery the
                    // map reads straight through the labels. Same material, more of it.
                    tint:         Qt.alpha(_qgcPal.window, 0.86)
                    minTint:      0.82
                    maxTint:      0.96
                }
            }

            Rectangle {
                id:                       grabber
                anchors.top:              parent.top
                anchors.topMargin:        ScreenTools.defaultFontPixelHeight * 0.35
                anchors.horizontalCenter: parent.horizontalCenter
                width:                    ScreenTools.defaultFontPixelWidth * 3.5
                height:                   ScreenTools.defaultFontPixelHeight * 0.2
                radius:                   height / 2
                color:                    Qt.alpha(_qgcPal.text, 0.28)
            }

            QGCMouseArea {
                anchors.top:              parent.top
                anchors.left:             parent.left
                anchors.right:            parent.right
                height:                   terrainSheet._grabberHeight
                onClicked:                terrainSheet.toggleVisible()
            }

            TerrainStatus {
                id:                 terrainStatus
                anchors.top:        parent.top
                anchors.topMargin:  terrainSheet._grabberHeight
                anchors.left:       parent.left
                anchors.right:      parent.right
                anchors.bottom:     parent.bottom
                color:              "transparent"
                border.width:       0
                missionController:  _missionController

                onSetCurrentSeqNum: _missionController.setCurrentPlanViewSeqNum(seqNum, true)
            }

            function toggleVisible() {
                _planViewSettings.showMissionItemStatus.rawValue = !_planViewSettings.showMissionItemStatus.rawValue
            }
        }

        MapScale {
            id:                     mapScale
            anchors.margins:        _toolsMargin
            anchors.bottom:         terrainSheet.visible ? terrainSheet.top : parent.bottom
            anchors.bottomMargin:   terrainSheet.visible ? _panelMargin + licenseLabel.height : _panelMargin
            anchors.left:           dock.right
            anchors.leftMargin:     _panelMargin
            mapControl:             editorMap
            buttonsOnLeft:          true
            terrainButtonVisible:   _editingLayer === _layerMission
            terrainButtonChecked:   _root._terrainProfileOpen
            onTerrainButtonClicked: terrainSheet.toggleVisible()
        }
    }

    function showLoadFromFileOverwritePrompt(title) {
        mainWindow.showMessageDialog(title,
                                     qsTr("You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?"),
                                     Dialog.Yes | Dialog.Cancel,
                                     function() { _planMasterController.loadFromSelectedFile() } )
    }

    Component {
        id: createPlanRemoveAllPromptDialog

        QGCSimpleMessageDialog {
            title:      qsTr("Create Plan")
            text:       qsTr("Are you sure you want to remove current plan and create a new plan? ")
            buttons:    Dialog.Yes | Dialog.No

            property var mapCenter
            property var planCreator

            onAccepted: planCreator.createPlan(mapCenter)
        }
    }

    function clearButtonClicked() {
        mainWindow.showMessageDialog(qsTr("Clear"),
                                     qsTr("Are you sure you want to remove all mission items and clear the mission from the vehicle?"),
                                     Dialog.Yes | Dialog.Cancel,
                                     function() { _planMasterController.removeAllFromVehicle();
                                                  _missionController.setCurrentPlanViewSeqNum(0, true);
                                                  if(_utmspEnabled)
                                                    {_resetRegisterFlightPlan = true;
                                                      QGroundControl.utmspManager.utmspVehicle.triggerActivationStatusBar(false);
                                                      UTMSPStateStorage.startTimeStamp = "";
                                                      UTMSPStateStorage.showActivationTab = false;
                                                      UTMSPStateStorage.flightID = "";
                                                      UTMSPStateStorage.enableMissionUploadButton = false;
                                                      UTMSPStateStorage.indicatorPendingStatus = true;
                                                      UTMSPStateStorage.indicatorApprovedStatus = false;
                                                      UTMSPStateStorage.indicatorActivatedStatus = false;
                                                      UTMSPStateStorage.currentStateIndex = 0}})
    }


    function downloadClicked(title) {
        if (_planMasterController.dirty) {
            mainWindow.showMessageDialog(title,
                                         qsTr("You have unsaved/unsent changes. Loading from the Vehicle will lose these changes. Are you sure you want to load from the Vehicle?"),
                                         Dialog.Yes | Dialog.Cancel,
                                         function() { _planMasterController.loadFromVehicle() })
        } else {
            _planMasterController.loadFromVehicle()
        }
    }


    Connections {
        target: utmspEditor
        function onVehicleIDSent(id) {
            _vehicleID = id
        }
    }
    Connections {
        target: utmspEditor
        function onRemoveFlightPlanTriggered() {
            _planMasterController.removeAllFromVehicle();
            _missionController.setCurrentPlanViewSeqNum(0, true);
            if(_utmspEnabled){_resetRegisterFlightPlan = true}
        }
    }
}
