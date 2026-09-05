/****************************************************************************
 *
 * (c) 2009-2020 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

import QtQuick
import QtLocation
import QtPositioning

import QGroundControl
import QGroundControl.Controls
import QGroundControl.FlightMap
import QGroundControl.ScreenTools
import QGroundControl.UTMSP

// The editable half of the plan, drawn onto a map this layer does not own.
//
// These visuals used to be declared inside the plan view's own FlightMap, which meant the plan
// needed a second map of its own - a second copy of the same ground, at its own centre, with its
// own backdrop. Everything here either takes a `map` or is handed to it as a MapItemGroup, so one
// map can carry the fly view's chrome and the plan's editing handles at the same time.
Item {
    id: _root

    property var  map
    property var  planMasterController
    // Map items attach to the map, not to this item, so hiding the plan view does not hide them.
    // Without this the plan's editable handles stayed on the map in fly mode, draggable.
    property bool planActive:           false
    property bool missionInteractive:   false
    property bool fenceInteractive:     false
    property bool rallyInteractive:     false
    property bool utmspInteractive:     false
    property bool utmspEnabled:         false
    property bool resetGeofencePolygon: false

    signal itemClicked(int sequenceNumber)

    readonly property var  _missionController:    planMasterController.missionController
    readonly property var  _geoFenceController:   planMasterController.geoFenceController
    readonly property var  _rallyPointController: planMasterController.rallyPointController
    readonly property real _dimmed:               0.5

    // MissionItemMapVisual and friends reparent their visuals onto the map and bind only opacity
    // through, so neither this item's visibility nor planActive can hide them - dimming them to
    // 0.5 left the plan's markers sitting on the fly view. Zero is the only value that removes
    // them by the one channel that reaches them.
    function _layerOpacity(interactive) { return !planActive ? 0 : (interactive ? 1 : _dimmed) }

    readonly property bool _editMission: planActive && missionInteractive
    readonly property bool _editFence:   planActive && fenceInteractive
    readonly property bool _editRally:   planActive && rallyInteractive
    readonly property bool _editUtmsp:   planActive && utmspInteractive

    property var _mapItemGroup

    // MapItemView and MapQuickItem only work as children of a Map, so the ones that cannot take a
    // `map` property are built into a group and handed over - the same trick PlanMapItems uses to
    // put a vehicle's mission on the fly view's map.
    // Keyed off the map arriving rather than off completion: a map bound from a parent that is
    // still being built is null at Component.onCompleted, and the group would then never be made.
    onMapChanged: _attachToMap()

    Component.onCompleted:   _attachToMap()
    Component.onDestruction: _detachFromMap()

    function _attachToMap() {
        if (_mapItemGroup || !map) {
            return
        }
        _mapItemGroup = mapItemGroupComponent.createObject(map)
        if (!_mapItemGroup || _mapItemGroup.status === Component.Error) {
            console.warn("PlanEditMapItems: cannot build map items:", mapItemGroupComponent.errorString())
            _mapItemGroup = null
            return
        }
        map.addMapItemGroup(_mapItemGroup)
    }

    function _detachFromMap() {
        if (!_mapItemGroup) {
            return
        }
        if (map) {
            // Must remove the group before destruction, otherwise we crash on quit
            map.removeMapItemGroup(_mapItemGroup)
        }
        _mapItemGroup.destroy()
        _mapItemGroup = null
    }

    Repeater {
        model: _missionController.visualItems

        delegate: MissionItemMapVisual {
            map:         _root.map
            opacity:     _root._layerOpacity(_root.missionInteractive)
            interactive: _root._editMission
            vehicle:     _root.planMasterController.controllerVehicle
            onClicked:   (sequenceNumber) => _root.itemClicked(sequenceNumber)
        }
    }

    GeoFenceMapVisuals {
        map:                    _root.map
        myGeoFenceController:   _root._geoFenceController
        interactive:            _root._editFence
        homePosition:           _root._missionController.plannedHomePosition
        planView:               true
        opacity:                _root._layerOpacity(_root.fenceInteractive)
    }

    RallyPointMapVisuals {
        map:                    _root.map
        myRallyPointController: _root._rallyPointController
        interactive:            _root._editRally
        planView:               true
        opacity:                _root._layerOpacity(_root.rallyInteractive)
    }

    UTMSPMapVisuals {
        enabled:                _root.utmspEnabled
        map:                    _root.map
        currentMissionItems:    _root._missionController.visualItems
        myGeoFenceController:   _root._geoFenceController
        interactive:            _root._editUtmsp
        homePosition:           _root._missionController.plannedHomePosition
        planView:               true
        opacity:                _root._layerOpacity(_root.utmspInteractive)
        resetCheck:             _root.resetGeofencePolygon
    }

    Component {
        id: mapItemGroupComponent

        MapItemGroup {
            visible: _root.planActive

            MissionLineView {
                showSpecialVisual:  _missionController.isROIBeginCurrentItem
                model:              _missionController.simpleFlightPathSegments
                opacity:            _root._editMission ? 1 : _root._dimmed
            }

            MapItemView {
                model: _missionController.directionArrows

                delegate: MapLineArrow {
                    visible:        _root._editMission
                    fromCoord:      object ? object.coordinate1 : undefined
                    toCoord:        object ? object.coordinate2 : undefined
                    arrowPosition:  3
                    z:              QGroundControl.zOrderWaypointLines + 1
                }
            }

            MapItemView {
                model: _missionController.incompleteComplexItemLines

                delegate: MapPolyline {
                    path:       [ object.coordinate1, object.coordinate2 ]
                    line.width: 1
                    line.color: "red"
                    z:          QGroundControl.zOrderWaypointLines
                    opacity:    _root._editMission ? 1 : _root._dimmed
                }
            }

            MapQuickItem {
                id:             splitSegmentItem
                anchorPoint.x:  sourceItem.width / 2
                anchorPoint.y:  sourceItem.height / 2
                z:              QGroundControl.zOrderWaypointLines + 1
                visible:        _root._editMission

                sourceItem: SplitIndicator {
                    onClicked:  _missionController.insertSimpleMissionItem(splitSegmentItem.coordinate,
                                                                           _missionController.currentPlanViewVIIndex,
                                                                           true /* makeCurrentItem */)
                }

                function _updateSplitCoord() {
                    if (_missionController.splitSegment) {
                        const distance = _missionController.splitSegment.coordinate1.distanceTo(_missionController.splitSegment.coordinate2)
                        const azimuth = _missionController.splitSegment.coordinate1.azimuthTo(_missionController.splitSegment.coordinate2)
                        splitSegmentItem.coordinate = _missionController.splitSegment.coordinate1.atDistanceAndAzimuth(distance / 2, azimuth)
                    } else {
                        coordinate = QtPositioning.coordinate()
                    }
                }

                Connections {
                    target: _missionController
                    function onSplitSegmentChanged() { splitSegmentItem._updateSplitCoord() }
                }

                Connections {
                    target: _missionController.splitSegment
                    function onCoordinate1Changed() { splitSegmentItem._updateSplitCoord() }
                    function onCoordinate2Changed() { splitSegmentItem._updateSplitCoord() }
                }
            }
        }
    }
}
