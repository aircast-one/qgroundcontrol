import CoreLocation
import CryptoKit
import MapLibre
import SwiftUI
import UIKit

private let VEHICLE_SOURCE = "aircast-vehicle"
private let TAP_SLOP_DP: CGFloat = 24

let NO_FADES = MLNTransition(duration: 0, delay: 0)
private let VEHICLE_LAYER = "aircast-vehicle-layer"
private let VEHICLE_HEADING_LAYER = "aircast-vehicle-heading-layer"
private let OTHER_VEHICLE_COLOUR = "#90A4AE"
private let VEHICLE_ARROW_IMAGE = "aircast-vehicle-arrow"

let HEADING_PROPERTY = "heading"
let ACTIVE_PROPERTY = "active"
let STALE_PROPERTY = "stale"
let VEHICLE_LABEL_PROPERTY = "vehicleLabel"
let VEHICLE_LABEL_LAYER = "aircast-vehicle-label"
private let TRAIL_SOURCE = "aircast-trail"
private let HOME_SOURCE = "aircast-home"
private let HOME_LAYER = "aircast-home-layer"
private let HOME_LABEL_LAYER = "aircast-home-label-layer"
private let TRAIL_LAYER = "aircast-trail-layer"

private let DEFAULT_ZOOM = 17.0

private let MIN_FIT_SPAN_DEGREES = 1e-5
private let FIT_PADDING_PIXELS: CGFloat = 80
private let FIT_SETTLE_MS = 100
private let LOGO_EDGE_MARGIN_PX: CGFloat = 16
private let ATTRIBUTION_CLEARANCE: CGFloat = 24
private let STALE_COLOUR = "#9E9E9E"

let OSM_TILE_URL = "https://tile.openstreetmap.org/{z}/{x}/{y}.png"

private let BLANK_STYLE = """
{ "version": 8, "sources": {}, "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf", "layers": [] }
"""

let OSM_RASTER_STYLE = """
{
  "version": 8,
  "sources": {
    "osm": {
      "type": "raster",
      "tiles": ["\(OSM_TILE_URL)"],
      "tileSize": 256,
      "attribution": "(c) OpenStreetMap contributors"
    }
  },
  "glyphs": "https://demotiles.maplibre.org/font/{fontstack}/{range}.pbf",
  "layers": [ { "id": "osm", "type": "raster", "source": "osm" } ]
}
"""

struct TrackPoint: Hashable {
    var latitude: Double
    var longitude: Double

    static func == (lhs: TrackPoint, rhs: TrackPoint) -> Bool {
        sameDouble(lhs.latitude, rhs.latitude) && sameDouble(lhs.longitude, rhs.longitude)
    }

    func hash(into hasher: inout Hasher) {
        hasher.combine(latitude.isNaN ? Double.nan : latitude)
        hasher.combine(longitude.isNaN ? Double.nan : longitude)
    }
}

extension TrackPoint {
    init(_ latitude: Double, _ longitude: Double) {
        self.init(latitude: latitude, longitude: longitude)
    }

    init(_ coordinate: CLLocationCoordinate2D) {
        self.init(latitude: coordinate.latitude, longitude: coordinate.longitude)
    }

    var location: CLLocationCoordinate2D { CLLocationCoordinate2D(latitude: latitude, longitude: longitude) }
}

enum FlightMapPosition {
    nonisolated(unsafe) static var latest: TrackPoint?
    nonisolated(unsafe) static var operatorCentred = false
}

func isPlottable(_ latitude: Double, _ longitude: Double) -> Bool {
    !latitude.isNaN && !longitude.isNaN
        && !(latitude == 0 && longitude == 0)
        && (-90.0...90.0).contains(latitude) && (-180.0...180.0).contains(longitude)
}

typealias Feature = MLNShape & MLNFeature
typealias FeatureCollection = MLNShapeCollectionFeature

func featureCollection(_ features: [Feature]) -> FeatureCollection {
    MLNShapeCollectionFeature(shapes: features)
}

func featureProperties(_ pairs: (String, Any?)...) -> [String: Any] {
    Dictionary(pairs.compactMap { key, value in value.map { (key, $0) } }, uniquingKeysWith: { _, last in last })
}

func pointFeature(_ at: TrackPoint, attributes: [String: Any] = [:]) -> Feature {
    let feature = MLNPointFeature()
    feature.coordinate = at.location
    feature.attributes = attributes
    return feature
}

func lineFeature(_ points: [TrackPoint], attributes: [String: Any] = [:]) -> Feature {
    let coordinates = points.map(\.location)
    let feature = MLNPolylineFeature(coordinates: coordinates, count: UInt(coordinates.count))
    feature.attributes = attributes
    return feature
}

func polygonFeature(_ ring: [TrackPoint], attributes: [String: Any] = [:]) -> Feature {
    let coordinates = ring.map(\.location)
    let feature = MLNPolygonFeature(coordinates: coordinates, count: UInt(coordinates.count))
    feature.attributes = attributes
    return feature
}

func multiLineFeature(_ lines: [[TrackPoint]]) -> Feature {
    MLNMultiPolylineFeature(polylines: lines.map { line in
        let coordinates = line.map(\.location)
        return MLNPolyline(coordinates: coordinates, count: UInt(coordinates.count))
    })
}

extension MLNFeature {
    func getStringProperty(_ key: String) -> String? { attribute(forKey: key) as? String }

    func getNumberProperty(_ key: String) -> Double? { (attribute(forKey: key) as? NSNumber)?.doubleValue }

    func getBooleanProperty(_ key: String) -> Bool { (attribute(forKey: key) as? NSNumber)?.boolValue ?? false }

    func hasProperty(_ key: String) -> Bool { attribute(forKey: key) != nil }
}

extension MLNMultiPoint {
    var trackPoints: [TrackPoint] {
        UnsafeBufferPointer(start: coordinates, count: Int(pointCount)).map(TrackPoint.init)
    }
}

extension MLNStyle {
    func setGeoJson(_ sourceId: String, _ features: FeatureCollection) {
        (source(withIdentifier: sourceId) as? MLNShapeSource)?.shape = features
    }
}

func geoJsonSource(_ identifier: String) -> MLNShapeSource {
    MLNShapeSource(identifier: identifier, shape: nil, options: nil)
}

func styleConstant(_ value: Any) -> NSExpression { NSExpression(forConstantValue: value) }

func styleExpression(_ json: Any) -> NSExpression { NSExpression(mglJSONObject: json) }

func styleGet(_ key: String) -> NSExpression { NSExpression(forKeyPath: key) }

func styleOffset(_ dx: CGFloat, _ dy: CGFloat) -> NSExpression {
    NSExpression(forConstantValue: NSValue(cgVector: CGVector(dx: dx, dy: dy)))
}

func mapColour(_ hex: String) -> UIColor {
    let digits = hex.removingPrefix("#")
    let value = UInt64(digits, radix: 16) ?? 0
    let channel = { (shift: UInt64) in CGFloat((value >> shift) & 0xFF) / 255 }
    return UIColor(red: channel(16), green: channel(8), blue: channel(0), alpha: digits.count > 6 ? channel(24) : 1)
}

func mapIcon(_ pixels: CGFloat, _ draw: (CGContext) -> Void) -> UIImage {
    let format = UIGraphicsImageRendererFormat()
    format.scale = 3
    return UIGraphicsImageRenderer(size: CGSize(width: pixels / 3, height: pixels / 3), format: format).image { rendered in
        rendered.cgContext.scaleBy(x: 1.0 / 3, y: 1.0 / 3)
        draw(rendered.cgContext)
    }
}

func vehicleFeatures(
    _ latitude: Double,
    _ longitude: Double,
    _ heading: Double,
    stale: Bool = false,
    lastSeen: String? = nil
) -> FeatureCollection {
    featureCollection(isPlottable(latitude, longitude) ? [vehicleFeature(latitude, longitude, heading, stale: stale, label: lastSeen)] : [])
}

func fleetFeatures(_ fleet: [VehicleChoice], lastSeen: String? = nil) -> FeatureCollection {
    featureCollection(
        fleet.filter { isPlottable($0.latitude, $0.longitude) }.map { flown in
            let label = [fleet.count > 1 ? "Vehicle \(flown.id)" : nil, flown.active ? lastSeen : nil]
                .compactMap { $0 }
                .joined(separator: "\n")
            return vehicleFeature(
                flown.latitude,
                flown.longitude,
                flown.heading,
                stale: flown.contactLost,
                active: flown.active,
                label: label.isEmpty ? nil : label
            )
        }
    )
}

func vehicleFeature(
    _ latitude: Double,
    _ longitude: Double,
    _ heading: Double,
    stale: Bool = false,
    active: Bool = true,
    label: String? = nil
) -> Feature {
    pointFeature(
        TrackPoint(latitude: latitude, longitude: longitude),
        attributes: featureProperties(
            (STALE_PROPERTY, stale),
            (ACTIVE_PROPERTY, active),
            (VEHICLE_LABEL_PROPERTY, label),
            (HEADING_PROPERTY, heading.isNaN ? nil : (heading.truncatingRemainder(dividingBy: 360) + 360).truncatingRemainder(dividingBy: 360))
        )
    )
}

struct VehicleMap: View {
    var mapStyle: String = OSM_RASTER_STYLE
    var follow: Bool = true
    var keepCentered: Bool = true
    var missionItems: [MissionItem] = []
    var linkStartToHome: Bool = false
    var fencePolygons: [FencePolygon] = []
    var fenceCircles: [FenceCircle] = []
    var rallyPoints: [RallyPoint] = []
    var `operator`: TrackPoint? = nil
    var operatorHeading: Double = .nan
    var surveys: [Survey] = []
    var shots: [TrackPoint] = []
    var landings: [LandingPattern] = []
    var editable: Bool = false
    var selectedWaypoint: Int? = nil
    var circledShapes: Set<String> = []
    var firmwareFence: FirmwareFence? = nil
    var onAdd: (Double, Double) -> Void = { _, _ in }
    var onBlankTap: ((Double, Double) -> Void)? = nil
    var onMove: (MapHit, Double, Double) -> Void = { _, _, _ in }
    var onWaypointSelected: (MapHit?) -> Void = { _ in }
    var onMoved: (MapHit, Double, Double) -> Void = { _, _, _ in }
    var canDrag: (MapHit) -> Bool = { _ in true }
    var onCentreChanged: (TrackPoint, Double) -> Void = { _, _ in }
    var onViewChanged: ([TrackPoint]) -> Void = { _ in }
    var bottomInsetPx: CGFloat = 0
    var topInsetPx: CGFloat = 0
    var leftInsetPx: CGFloat = 0
    var rightInsetPx: CGFloat = 0
    var logoEndInsetPx: CGFloat? = nil
    var cameraBottomPx: CGFloat = 0
    var pip: Bool = false
    var fitRequest: Int = 0
    var fitOnly: [TrackPoint]? = nil
    var onFitFailed: () -> Void = {}
    var centreRequest: Int = 0
    var centreOn: TrackPoint? = nil
    var centreZoom: Double? = nil
    var gestures: Bool = true
    var onMapClick: ((Double, Double) -> Void)? = nil
    var onMissionItemClick: ((Int) -> Void)? = nil
    var traffic: [TrafficMark] = []
    var onTrafficClick: (() -> Void)? = nil
    var gimbals: [GimbalAzimuth] = []
    var breachReturn: TrackPoint? = nil
    var proximityRadar: Bool = false
    var obstacleOverlay: Bool = false
    var tracePoints: [TrackPoint] = []
    var traceLine: Bool = false
    var roi: TrackPoint? = nil
    var onRoiClick: ((TrackPoint) -> Void)? = nil
    var goto: GotoLocation? = nil
    var clickMarker: TrackPoint? = nil
    var collisionLegs: [(TrackPoint, TrackPoint)] = []
    var orbit: OrbitCircle? = nil
    var otherMissions: [OtherMission] = []

    @Environment(FlyMapEdits.self) private var mapEdits
    @MapViewFlag(FLY_STATE_VIEW, "contactLost") private var linkLost
    @MapPath(VEHICLES_VIEW) private var fleetJson
    @MapPath(TRACK_TAIL_VIEW) private var trackJson
    @StateObject private var model = VehicleMapModel()
    @State private var track = trackReading(nil)

    var body: some View {
        SilentSeconds(lost: linkLost) { seconds in
            drawn(VehicleMapScene(self, lastSeen: seconds.map(lastSeenText), mapEdits: mapEdits, fleetJson: fleetJson, linkLost: linkLost))
        }
        .task(id: trackJson) { await mergeTrack(trackJson) }
    }

    private func mergeTrack(_ tailJson: JSON?) async {
        let tail = trackReading(tailJson)
        if let merged = mergedTrack(track, tail) {
            track = merged
            return
        }
        let full = await offMain { MapBridge.read(TRACK_VIEW) }
        guard !Task.isCancelled else { return }
        track = full.map { trackReading($0) } ?? withChanges(tail) { $0.points = []; $0.from = 0 }
    }

    private func drawn(_ scene: VehicleMapScene) -> some View {
        ZStack {
            VehicleMapView(
                model: model,
                inputs: scene.inputs,
                mapStyle: mapStyle,
                pip: pip,
                cameraBottomPx: cameraBottomPx,
                bottomInsetPx: bottomInsetPx,
                leftInsetPx: leftInsetPx,
                logoEndInsetPx: logoEndInsetPx,
                gestures: gestures
            )
            if obstacleOverlay {
                ObstacleMapOverlay(camera: model.camera, latitude: scene.latitude, longitude: scene.longitude, heading: scene.heading, showText: !pip)
            }
        }
        .modifier(VehicleMapOverlays(map: self, scene: scene, model: model))
        .modifier(VehicleMapPlan(map: self, scene: scene, model: model, track: track))
    }
}

private struct VehicleMapScene {
    let shownGoto: GotoLocation?
    let gotoEditing: Bool
    let lastSeen: String?
    let linkLost: Bool
    let fleetJson: JSON?
    let fleet: [VehicleChoice]
    let latitude: Double
    let longitude: Double
    let heading: Double
    let home: TrackPoint?
    let radars: [PlacedRadar]
    let orbitPreview: OrbitCircle?
    let inputs: VehicleMapInputs

    init(_ map: VehicleMap, lastSeen: String?, mapEdits: FlyMapEdits, fleetJson: JSON?, linkLost: Bool) {
        let shownGoto = editedGoto(map.goto, mapEdits.gotoLoiter)
        let fleet = vehicleChoices(fleetJson).choices
        let flown = fleet.first(where: \.active)
        let latitude = flown?.latitude ?? .nan
        let longitude = flown?.longitude ?? .nan
        let heading = flown?.heading ?? .nan
        self.shownGoto = shownGoto
        self.gotoEditing = mapEdits.gotoLoiter != nil && shownGoto?.loiterRadiusMetres != nil
        self.lastSeen = lastSeen
        self.linkLost = linkLost
        self.fleetJson = fleetJson
        self.fleet = fleet
        self.latitude = latitude
        self.longitude = longitude
        self.heading = heading
        self.home = flown?.home
        self.radars = map.proximityRadar ? placedRadars(fleet, TrackPoint(latitude: latitude, longitude: longitude), heading) : []
        self.orbitPreview = mapEdits.orbit
        self.inputs = VehicleMapInputs(
            follow: map.follow,
            keepCentered: map.keepCentered,
            latitude: latitude,
            longitude: longitude,
            topInsetPx: map.topInsetPx,
            bottomInsetPx: map.bottomInsetPx,
            leftInsetPx: map.leftInsetPx,
            rightInsetPx: map.rightInsetPx,
            cameraBottomPx: map.cameraBottomPx,
            editable: map.editable,
            missionItems: map.missionItems,
            roi: map.roi,
            goto: shownGoto,
            onMapClick: map.onMapClick,
            onMissionItemClick: map.onMissionItemClick,
            onRoiClick: map.onRoiClick,
            onTrafficClick: map.onTrafficClick,
            onAdd: map.onAdd,
            onBlankTap: map.onBlankTap,
            onMove: map.onMove,
            onWaypointSelected: map.onWaypointSelected,
            onMoved: map.onMoved,
            canDrag: map.canDrag,
            onCentreChanged: map.onCentreChanged,
            onViewChanged: map.onViewChanged,
            edits: mapEdits
        )
    }

    var placed: TrackPoint? { isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil }

    var headingKey: Double? { heading.isNaN ? nil : heading }
}

private struct VehicleMapOverlays: ViewModifier {
    let map: VehicleMap
    let scene: VehicleMapScene
    @ObservedObject var model: VehicleMapModel

    func body(content: Content) -> some View {
        let generation = model.generation
        let style = model.style
        return content
            .onChange(of: Keys(generation, model.draggingVertex, map.fencePolygons, map.surveys), initial: true) {
                style?.setGeoJson(
                    EDGE_LABEL_SOURCE,
                    featureCollection(edgeLabels(model.draggingVertex, map.fencePolygons, map.surveys).map { label in
                        pointFeature(label.at, attributes: [LANDING_LABEL_TEXT: label.text])
                    })
                )
            }
            .onChange(of: Keys(generation, map.traffic), initial: true) {
                style?.setGeoJson(TRAFFIC_SOURCE, trafficFeatures(map.traffic))
            }
            .onChange(of: Keys(generation, scene.radars), initial: true) {
                if let style { renderProximityRadars(style, scene.radars) }
            }
            .onChange(of: Keys(generation, map.orbit, scene.shownGoto, scene.orbitPreview), initial: true) {
                if let style { renderOrbit(style, map.orbit, scene.shownGoto != nil, preview: scene.orbitPreview) }
            }
            .onChange(of: Keys(generation, map.collisionLegs.flatMap { [$0.0, $0.1] }), initial: true) {
                if let style { renderCollisionLegs(style, map.collisionLegs) }
            }
            .onChange(of: Keys(generation, map.clickMarker, scene.orbitPreview), initial: true) {
                if let style { renderClickMarker(style, scene.orbitPreview == nil ? map.clickMarker : nil) }
            }
            .onChange(of: Keys(generation, scene.shownGoto, scene.gotoEditing), initial: true) {
                if let style { renderGoto(style, scene.shownGoto, editing: scene.gotoEditing) }
            }
            .onChange(of: Keys(generation, map.roi), initial: true) {
                if let style { renderRoi(style, map.roi) }
            }
            .onChange(of: Keys(generation, map.tracePoints, map.traceLine), initial: true) {
                if let style { renderTrace(style, map.tracePoints, map.traceLine) }
            }
            .onChange(of: Keys(generation, scene.placed, map.gimbals), initial: true) {
                if let style { renderGimbals(style, scene.latitude, scene.longitude, map.gimbals) }
            }
            .onChange(of: Keys(generation, map.shots), initial: true) {
                style?.setGeoJson(SHOT_SOURCE, shotFeatures(map.shots))
            }
    }
}

private struct VehicleMapPlan: ViewModifier {
    let map: VehicleMap
    let scene: VehicleMapScene
    @ObservedObject var model: VehicleMapModel
    let track: TrackReading

    func body(content: Content) -> some View {
        let generation = model.generation
        let style = model.style
        return content
            .onChange(of: Keys(generation, scene.placed, scene.headingKey, scene.home, scene.linkLost, scene.fleetJson, scene.lastSeen), initial: true) {
                guard let style else { return }
                style.setGeoJson(
                    VEHICLE_SOURCE,
                    scene.fleet.isEmpty
                        ? vehicleFeatures(scene.latitude, scene.longitude, scene.heading, stale: scene.linkLost, lastSeen: scene.lastSeen)
                        : fleetFeatures(scene.fleet, lastSeen: scene.lastSeen)
                )
                style.setGeoJson(
                    HOME_SOURCE,
                    featureCollection((scene.fleet.isEmpty ? [scene.home].compactMap { $0 } : scene.fleet.compactMap(\.home)).map { pointFeature($0) })
                )
                model.recenterOnVehicle(scene.inputs)
            }
            .onChange(of: Keys(generation, track), initial: true) {
                style?.setGeoJson(TRAIL_SOURCE, featureCollection(trackDraws(track) ? [lineFeature(plottedTrack(track))] : []))
            }
            .onChange(of: Keys(generation, map.pip), initial: true) {
                if let style { applyPip(style, map.pip) }
            }
            .onChange(of: Keys(model.mapReady, map.centreRequest), initial: true) {
                model.centre(map.centreRequest, map.centreOn, map.centreZoom, UIEdgeInsets(top: map.topInsetPx, left: map.leftInsetPx, bottom: map.bottomInsetPx, right: 0))
            }
            .task(id: Keys(map.fitRequest, map.topInsetPx, map.leftInsetPx, map.bottomInsetPx)) {
                guard map.fitRequest != 0, map.fitRequest != model.fittedRequest else { return }
                try? await Task.sleep(for: .milliseconds(FIT_SETTLE_MS))
                guard !Task.isCancelled else { return }
                model.fittedRequest = map.fitRequest
                let settled = model.inputs
                model.fit(
                    map.fitOnly ?? planPoints(map.missionItems, map.fencePolygons, map.fenceCircles, map.rallyPoints, map.surveys),
                    scene.latitude,
                    scene.longitude,
                    UIEdgeInsets(top: settled.topInsetPx, left: settled.leftInsetPx, bottom: settled.bottomInsetPx, right: 0),
                    map.onFitFailed
                )
            }
            .onChange(
                of: Keys(
                    generation, map.missionItems, map.fencePolygons, map.fenceCircles, map.rallyPoints, map.surveys,
                    map.landings, map.firmwareFence, map.selectedWaypoint, map.linkStartToHome, map.operator,
                    map.operatorHeading.isNaN ? nil : map.operatorHeading, map.breachReturn, map.otherMissions, map.circledShapes
                ),
                initial: true
            ) {
                if let style { renderPlan(style) }
            }
    }

    private func renderPlan(_ style: MLNStyle) {
        let editedItems = map.editable ? map.missionItems : []
        renderSurveys(style, map.surveys)
        renderTransectMarks(
            style,
            map.surveys,
            map.selectedWaypoint,
            legArrows: legArrows(map.missionItems, map.linkStartToHome) + loiterRotationArrows(editedItems)
        )
        renderGimbalWedges(style, map.missionItems)
        renderLandings(style, map.landings, editedItems, map.editable ? map.selectedWaypoint : nil)
        renderMidpoints(
            style,
            map.fencePolygons.map { fence in
                map.circledShapes.contains(fencePath(fence.index)) ? withChanges(fence) { $0.editable = nil } : fence
            },
            map.surveys.map { survey in
                map.circledShapes.contains(surveyPath(survey)) ? withChanges(survey) { $0.editable = nil } : survey
            },
            items: editedItems,
            selected: map.selectedWaypoint
        )
        renderFences(
            style,
            map.fencePolygons,
            map.rallyPoints,
            circles: circlesAsPolygons(map.fenceCircles),
            firmware: map.firmwareFence,
            breach: map.breachReturn.flatMap { isPlottable($0.latitude, $0.longitude) ? $0 : nil }
        )
        style.setGeoJson(GCS_SOURCE, operatorFeatures(map.operator, heading: map.operatorHeading))
        renderVertexHandles(
            style,
            map.fencePolygons,
            map.surveys,
            circles: map.fenceCircles,
            landings: map.landings,
            circled: map.circledShapes,
            loiterHandles: loiterHandleFeatures(editedItems, map.selectedWaypoint)
        )
        renderMission(style, map.missionItems, map.linkStartToHome, selectedIndex: map.selectedWaypoint, others: map.otherMissions, landings: map.landings, legs: map.editable)
    }
}

struct VehicleMapInputs {
    var follow = true
    var keepCentered = true
    var latitude = Double.nan
    var longitude = Double.nan
    var topInsetPx: CGFloat = 0
    var bottomInsetPx: CGFloat = 0
    var leftInsetPx: CGFloat = 0
    var rightInsetPx: CGFloat = 0
    var cameraBottomPx: CGFloat = 0
    var editable = false
    var missionItems: [MissionItem] = []
    var roi: TrackPoint?
    var goto: GotoLocation?
    var onMapClick: ((Double, Double) -> Void)?
    var onMissionItemClick: ((Int) -> Void)?
    var onRoiClick: ((TrackPoint) -> Void)?
    var onTrafficClick: (() -> Void)?
    var onAdd: (Double, Double) -> Void = { _, _ in }
    var onBlankTap: ((Double, Double) -> Void)?
    var onMove: (MapHit, Double, Double) -> Void = { _, _, _ in }
    var onWaypointSelected: (MapHit?) -> Void = { _ in }
    var onMoved: (MapHit, Double, Double) -> Void = { _, _, _ in }
    var canDrag: (MapHit) -> Bool = { _ in true }
    var onCentreChanged: (TrackPoint, Double) -> Void = { _, _ in }
    var onViewChanged: ([TrackPoint]) -> Void = { _ in }
    var edits: FlyMapEdits?
}

final class MapCamera: ObservableObject {
    @Published fileprivate(set) var moves = 0
    weak var mapView: MLNMapView?
}

private let PAN_RECENTER_DELAY_MS = 10_000.0
private let RECENTER_ANIMATION_MS = 1000.0
private let CENTRE_INSET_FRACTION: CGFloat = 0.15

private let gestureReasons: MLNCameraChangeReason = [
    .gesturePan, .gesturePinch, .gestureRotate, .gestureZoomIn, .gestureZoomOut, .gestureOneFingerZoom, .gestureTilt,
]

final class VehicleMapModel: NSObject, ObservableObject, MLNMapViewDelegate {
    @Published fileprivate(set) var style: MLNStyle?
    @Published fileprivate(set) var generation = 0
    @Published fileprivate(set) var mapReady = false
    @Published fileprivate(set) var draggingVertex: MapHit?
    let camera = MapCamera()
    fileprivate var inputs = VehicleMapInputs()
    fileprivate var appliedGestures: Bool?
    fileprivate var fittedRequest = 0
    private var appliedStyle = ""
    private var panning = false
    private var trackingResumesAt = Date.distantPast
    private var resume: Task<Void, Never>?
    private var gesturesAttached = false

    var mapView: MLNMapView? { camera.mapView }

    fileprivate func attach(_ view: MLNMapView) {
        camera.mapView = view
        view.delegate = self
        DispatchQueue.main.async { self.mapReady = true }
    }

    fileprivate func initialStyleURL(_ mapStyle: String) -> URL? {
        let url = mapStyle.isBlank ? nil : styleURL(mapStyle)
        appliedStyle = url == nil ? "" : mapStyle
        return url ?? styleURL(BLANK_STYLE)
    }

    fileprivate func applyStyle(_ mapStyle: String, _ view: MLNMapView) {
        guard mapStyle != appliedStyle, !mapStyle.isBlank, let url = styleURL(mapStyle) else { return }
        appliedStyle = mapStyle
        view.styleURL = url
    }

    func mapView(_ mapView: MLNMapView, didFinishLoading style: MLNStyle) {
        style.transition = NO_FADES
        style.performsPlacementTransitions = false
        installLayers(style)
        installSurveyLayers(style)
        installTransectMarks(style)
        installLandingLayers(style)
        installMidpointLayer(style)
        installShotLayer(style)
        installFenceLayers(style)
        installMissionLayers(style)
        installTraceLayer(style)
        installFenceHandleLayer(style)
        installVehicleLayer(style)
        installTrafficLayer(style)
        installRoiLayer(style)
        installGotoLayer(style)
        installClickMarker(style)
        installOrbitLayer(style)
        attachGestures(mapView)
        reportCentre(mapView)
        draggingVertex = nil
        self.style = style
        generation += 1
    }

    func mapView(_ mapView: MLNMapView, regionWillChangeWith reason: MLNCameraChangeReason, animated: Bool) {
        if !reason.isDisjoint(with: gestureReasons) { panning = true }
    }

    func mapViewRegionIsChanging(_ mapView: MLNMapView) {
        camera.moves += 1
    }

    func mapView(_ mapView: MLNMapView, regionDidChangeAnimated animated: Bool) {
        camera.moves += 1
        reportCentre(mapView)
        guard panning else { return }
        panning = false
        trackingResumesAt = Date().addingTimeInterval(PAN_RECENTER_DELAY_MS / 1000)
        resume?.cancel()
        resume = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(PAN_RECENTER_DELAY_MS))
            guard !Task.isCancelled, let self else { return }
            self.recenterOnVehicle(self.inputs)
        }
    }

    private func reportCentre(_ mapView: MLNMapView) {
        inputs.onCentreChanged(TrackPoint(mapView.centerCoordinate), mapView.zoomLevel)
        let right = max(mapView.bounds.width - inputs.rightInsetPx, 0)
        let left = inputs.leftInsetPx.clamped(to: 0...right)
        let bottom = max(mapView.bounds.height - inputs.bottomInsetPx, 0)
        let top = inputs.topInsetPx.clamped(to: 0...bottom)
        let seen = [CGPoint(x: left, y: top), CGPoint(x: right, y: top), CGPoint(x: right, y: bottom), CGPoint(x: left, y: bottom)]
            .map { TrackPoint(mapView.convert($0, toCoordinateFrom: mapView)) }
        inputs.onViewChanged(clearWindow(seen))
    }

    private func attachGestures(_ mapView: MLNMapView) {
        guard !gesturesAttached else { return }
        gesturesAttached = true
        if !inputs.editable && inputs.onMapClick != nil {
            let tap = UITapGestureRecognizer(target: self, action: #selector(tapped(_:)))
            mapView.gestureRecognizers?
                .compactMap { $0 as? UITapGestureRecognizer }
                .filter { $0.numberOfTapsRequired == 2 }
                .forEach(tap.require(toFail:))
            mapView.addGestureRecognizer(tap)
            mapView.addGestureRecognizer(UILongPressGestureRecognizer(target: self, action: #selector(longPressed(_:))))
            if let edits = inputs.edits {
                attachGotoRadiusDrag(mapView, edits) { [weak self] in self?.inputs.goto }
            }
        }
        if inputs.editable {
            attachMissionEditing(
                mapView,
                onAdd: { [weak self] latitude, longitude in self?.inputs.onAdd(latitude, longitude) },
                onMove: { [weak self] hit, latitude, longitude in self?.inputs.onMove(hit, latitude, longitude) },
                onSelected: { [weak self] hit in self?.inputs.onWaypointSelected(hit) },
                onMoved: { [weak self] hit, latitude, longitude in self?.inputs.onMoved(hit, latitude, longitude) },
                onDragging: { [weak self] hit in self?.draggingVertex = hit },
                canDrag: { [weak self] hit in self?.inputs.canDrag(hit) ?? true },
                onBlankTap: { [weak self] latitude, longitude in
                    guard let self else { return }
                    if let blank = self.inputs.onBlankTap { blank(latitude, longitude) } else { self.inputs.onWaypointSelected(nil) }
                }
            )
        }
    }

    @objc private func tapped(_ tap: UITapGestureRecognizer) {
        guard tap.state == .ended, let mapView else { return }
        let screen = tap.location(in: mapView)
        let near = CGRect(x: screen.x - TAP_SLOP_DP, y: screen.y - TAP_SLOP_DP, width: TAP_SLOP_DP * 2, height: TAP_SLOP_DP * 2)
        if let trafficClick = inputs.onTrafficClick, !mapView.visibleFeatures(in: near, styleLayerIdentifiers: [TRAFFIC_LAYER]).isEmpty {
            return trafficClick()
        }
        if let roi = inputs.roi, let roiClick = inputs.onRoiClick, !mapView.visibleFeatures(in: near, styleLayerIdentifiers: [ROI_LAYER]).isEmpty {
            return roiClick(roi)
        }
        guard case .Waypoint(let index)? = hitTest(mapView, Float(screen.x), Float(screen.y)),
              let item = inputs.missionItems.first(where: { $0.index == index }),
              let itemClick = inputs.onMissionItemClick else { return }
        itemClick(item.sequence)
    }

    @objc private func longPressed(_ press: UILongPressGestureRecognizer) {
        guard press.state == .began, let mapView else { return }
        let at = mapView.convert(press.location(in: mapView), toCoordinateFrom: mapView)
        inputs.onMapClick?(at.latitude, at.longitude)
    }

    fileprivate func recenterOnVehicle(_ inputs: VehicleMapInputs) {
        let tracking = inputs.follow && isPlottable(inputs.latitude, inputs.longitude) && !panning && Date() >= trackingResumesAt
        guard tracking, let mapView else { return }
        let at = CLLocationCoordinate2D(latitude: inputs.latitude, longitude: inputs.longitude)
        let zoomed = mapView.zoomLevel > 1.0
        let point = mapView.convert(at, toPointTo: mapView)
        let height = mapView.bounds.height
        let lift = clearAreaLift(inputs.topInsetPx, inputs.bottomInsetPx).clamped(to: -height / 4...height / 4)
        let centred = zoomed && lift != 0 ? mapView.convert(CGPoint(x: point.x, y: point.y - lift), toCoordinateFrom: mapView) : at
        if inputs.keepCentered || !zoomed {
            mapView.setCenter(centred, zoomLevel: zoomed ? mapView.zoomLevel : DEFAULT_ZOOM, animated: false)
        } else if outsideCentreInset(point.x, point.y, mapView.bounds.width, height, inputs.topInsetPx, inputs.bottomInsetPx + inputs.cameraBottomPx) {
            let camera = withChanges(mapView.camera) { $0.centerCoordinate = centred }
            mapView.setCamera(camera, withDuration: RECENTER_ANIMATION_MS / 1000, animationTimingFunction: nil)
        }
    }

    fileprivate func centre(_ request: Int, _ centreOn: TrackPoint?, _ centreZoom: Double?, _ insets: UIEdgeInsets) {
        guard request != 0, let at = centreOn, isPlottable(at.latitude, at.longitude), let mapView else { return }
        if let centreZoom {
            mapView.setCenter(at.location, zoomLevel: centreZoom, animated: false)
        } else if mapView.zoomLevel > 1.0 {
            let point = mapView.convert(at.location, toPointTo: mapView)
            let clear = CGPoint(x: point.x - insets.left / 2, y: point.y - clearAreaLift(insets.top, insets.bottom))
            mapView.setCenter(mapView.convert(clear, toCoordinateFrom: mapView), animated: true)
        } else {
            mapView.setCenter(at.location, zoomLevel: DEFAULT_ZOOM, animated: true)
        }
    }

    fileprivate func fit(_ points: [TrackPoint], _ latitude: Double, _ longitude: Double, _ insets: UIEdgeInsets, _ onFitFailed: () -> Void) {
        guard let mapView else { return }
        guard let bounds = planBounds(fitPoints(points, latitude, longitude)) else { return onFitFailed() }
        if bounds.spanDegrees < MIN_FIT_SPAN_DEGREES {
            return mapView.setCenter(bounds.centre.location, zoomLevel: DEFAULT_ZOOM, animated: true)
        }
        let padding = FIT_PADDING_PIXELS / max(mapView.traitCollection.displayScale, 1)
        mapView.setVisibleCoordinateBounds(
            MLNCoordinateBounds(
                sw: CLLocationCoordinate2D(latitude: bounds.south, longitude: bounds.west),
                ne: CLLocationCoordinate2D(latitude: bounds.north, longitude: bounds.east >= bounds.west ? bounds.east : bounds.east + 360)
            ),
            edgePadding: UIEdgeInsets(
                top: padding + insets.top,
                left: padding + insets.left,
                bottom: padding + insets.bottom,
                right: padding
            ),
            animated: true,
            completionHandler: nil
        )
    }
}

func styleURL(_ mapStyle: String) -> URL? {
    guard mapStyle.trimmed.hasPrefix("{") else { return URL(string: mapStyle) }
    let digest = SHA256.hash(data: Data(mapStyle.utf8)).map { String(format: "%02x", $0) }.joined()
    return writtenStyleURL(mapStyle, "inline-\(digest)")
}

private struct VehicleMapView: UIViewRepresentable {
    let model: VehicleMapModel
    let inputs: VehicleMapInputs
    let mapStyle: String
    let pip: Bool
    let cameraBottomPx: CGFloat
    let bottomInsetPx: CGFloat
    let leftInsetPx: CGFloat
    let logoEndInsetPx: CGFloat?
    let gestures: Bool

    func makeUIView(context: Context) -> MLNMapView {
        let view = MLNMapView(frame: .zero, styleURL: model.initialStyleURL(mapStyle))
        view.automaticallyAdjustsContentInset = false
        model.inputs = inputs
        model.attach(view)
        return view
    }

    func updateUIView(_ view: MLNMapView, context: Context) {
        model.inputs = inputs
        model.applyStyle(mapStyle, view)
        view.logoView.isHidden = pip
        view.attributionButton.isHidden = pip
        let inset = UIEdgeInsets(top: 0, left: 0, bottom: cameraBottomPx, right: 0)
        if view.contentInset != inset {
            view.setContentInset(inset, animated: false, completionHandler: nil)
        }
        let safe = view.safeAreaInsets
        let bottom = max(0, bottomInsetPx + LOGO_EDGE_MARGIN_PX - safe.bottom)
        let slack = max(0, (MINIMUM_TOUCH_TARGET - view.attributionButton.intrinsicContentSize.width) / 2)
        if let end = logoEndInsetPx {
            view.logoViewPosition = .bottomRight
            view.attributionButtonPosition = .bottomRight
            view.attributionButtonMargins = CGPoint(x: max(0, end - safe.right) - slack, y: bottom - slack)
            view.logoViewMargins = CGPoint(x: max(0, end + ATTRIBUTION_CLEARANCE - safe.right), y: bottom)
        } else {
            let left = max(0, leftInsetPx + LOGO_EDGE_MARGIN_PX - safe.left)
            view.logoViewPosition = .bottomLeft
            view.attributionButtonPosition = .bottomLeft
            view.logoViewMargins = CGPoint(x: left, y: bottom)
            view.attributionButtonMargins = CGPoint(x: left + view.logoView.bounds.width + Space.s1 - slack, y: bottom - slack)
        }
        view.updateConstraintsIfNeeded()
        view.attributionButton.constraints
            .filter { $0.firstAttribute == .width || $0.firstAttribute == .height }
            .forEach { $0.constant = MINIMUM_TOUCH_TARGET }
        if model.appliedGestures != gestures {
            model.appliedGestures = gestures
            view.isScrollEnabled = gestures
            view.isZoomEnabled = gestures
            view.isRotateEnabled = gestures
            view.isPitchEnabled = gestures
        }
    }
}

private struct ObstacleMapOverlay: View {
    @ObservedObject var camera: MapCamera
    let latitude: Double
    let longitude: Double
    let heading: Double
    let showText: Bool
    @MapPath(OBSTACLE_VIEW) private var json

    var body: some View {
        if let overlay = obstacleOverlay(json) {
            Canvas { context, size in
                guard let map = camera.mapView, isPlottable(latitude, longitude), !heading.isNaN else { return }
                let vehicle = map.convert(TrackPoint(latitude: latitude, longitude: longitude).location, toPointTo: map)
                let left = map.convert(.zero, toCoordinateFrom: map)
                let right = map.convert(CGPoint(x: TRUE_SCALE_PROBE, y: 0), toCoordinateFrom: map)
                let metresInProbe = CLLocation(latitude: left.latitude, longitude: left.longitude)
                    .distance(from: CLLocation(latitude: right.latitude, longitude: right.longitude))
                let shape = mapOverlayShape(overlay, vehicle, size.height, metresInProbe, TRUE_SCALE_PROBE, heading, map.direction)
                drawMapObstacleOverlay(context, overlay, shape, vehicle, showText)
            }
            .allowsHitTesting(false)
        }
    }
}

private func installLayers(_ style: MLNStyle) {
    if style.source(withIdentifier: TRAIL_SOURCE) == nil {
        let source = geoJsonSource(TRAIL_SOURCE)
        style.addSource(source)
        let trail = MLNLineStyleLayer(identifier: TRAIL_LAYER, source: source)
        trail.lineColor = styleConstant(mapColour("#FF0000"))
        trail.lineWidth = styleConstant(3)
        style.addLayer(trail)
    }

    if style.source(withIdentifier: HOME_SOURCE) == nil {
        let source = geoJsonSource(HOME_SOURCE)
        style.addSource(source)
        let dot = MLNCircleStyleLayer(identifier: HOME_LAYER, source: source)
        dot.circleColor = styleConstant(mapColour("#43A047"))
        dot.circleRadius = styleConstant(12)
        dot.circleStrokeColor = styleConstant(UIColor.white)
        dot.circleStrokeWidth = styleConstant(2)
        style.addLayer(dot)
        let label = MLNSymbolStyleLayer(identifier: HOME_LABEL_LAYER, source: source)
        label.text = styleConstant("H")
        label.textFontNames = styleConstant(["Noto Sans Regular"])
        label.textFontSize = styleConstant(14)
        label.textColor = styleConstant(UIColor.white)
        label.textAllowsOverlap = styleConstant(true)
        label.textIgnoresPlacement = styleConstant(true)
        style.addLayer(label)
    }
}

let PIP_ICON_SCALE: CGFloat = 1.0 / 3.0
let PIP_TRAFFIC_SCALE: CGFloat = 1.0 / 2.5

private func applyPip(_ style: MLNStyle, _ pip: Bool) {
    style.layer(withIdentifier: TRAIL_LAYER)?.isVisible = !pip
    let scale = pip ? PIP_ICON_SCALE : 1
    if let vehicle = style.layer(withIdentifier: VEHICLE_LAYER) as? MLNCircleStyleLayer {
        vehicle.circleRadius = styleExpression(["case", ["get", ACTIVE_PROPERTY], 9 * scale, 6 * scale])
        vehicle.circleStrokeWidth = styleConstant(2 * scale)
    }
    (style.layer(withIdentifier: VEHICLE_HEADING_LAYER) as? MLNSymbolStyleLayer)?.iconScale = styleConstant(scale)
    if let aircraft = style.layer(withIdentifier: TRAFFIC_LAYER) as? MLNSymbolStyleLayer {
        aircraft.iconScale = styleConstant(pip ? PIP_TRAFFIC_SCALE : 1)
        aircraft.textFontSize = styleConstant(pip ? 11 * PIP_TRAFFIC_SCALE : 11)
    }
}

private func installVehicleLayer(_ style: MLNStyle) {
    installProximityRadarLayer(style)
    installGimbalLayer(style)
    guard style.source(withIdentifier: VEHICLE_SOURCE) == nil else { return }
    let source = geoJsonSource(VEHICLE_SOURCE)
    style.addSource(source)
    let dot = MLNCircleStyleLayer(identifier: VEHICLE_LAYER, source: source)
    dot.circleColor = styleExpression([
        "case",
        ["get", STALE_PROPERTY], STALE_COLOUR,
        ["==", ["get", ACTIVE_PROPERTY], false], OTHER_VEHICLE_COLOUR,
        "#E53935",
    ])
    dot.circleRadius = styleExpression(["case", ["get", ACTIVE_PROPERTY], 9, 6])
    dot.circleStrokeColor = styleConstant(UIColor.white)
    dot.circleStrokeWidth = styleConstant(2)
    style.addLayer(dot)
    style.setImage(headingArrow(), forName: VEHICLE_ARROW_IMAGE)
    let arrow = MLNSymbolStyleLayer(identifier: VEHICLE_HEADING_LAYER, source: source)
    arrow.iconImageName = styleConstant(VEHICLE_ARROW_IMAGE)
    arrow.iconRotation = styleGet(HEADING_PROPERTY)
    arrow.iconRotationAlignment = styleConstant("map")
    arrow.iconOpacity = styleExpression(["case", ["get", STALE_PROPERTY], 0.4, 1.0])
    arrow.iconAllowsOverlap = styleConstant(true)
    arrow.iconIgnoresPlacement = styleConstant(true)
    arrow.predicate = NSPredicate(format: "%K != nil", HEADING_PROPERTY)
    style.addLayer(arrow)
    let label = MLNSymbolStyleLayer(identifier: VEHICLE_LABEL_LAYER, source: source)
    label.text = styleGet(VEHICLE_LABEL_PROPERTY)
    label.textFontSize = styleConstant(11)
    label.textColor = styleConstant(UIColor.white)
    label.textHaloColor = styleConstant(UIColor.black)
    label.textHaloWidth = styleConstant(1)
    label.textOffset = styleOffset(0, 1.6)
    label.textAnchor = styleConstant("top")
    label.textAllowsOverlap = styleConstant(true)
    label.textIgnoresPlacement = styleConstant(false)
    label.predicate = NSPredicate(format: "%K != nil", VEHICLE_LABEL_PROPERTY)
    style.addLayer(label)
}

private func headingArrow() -> UIImage {
    let size: CGFloat = 48
    return mapIcon(size) { context in
        let path = CGMutablePath()
        path.move(to: CGPoint(x: size / 2, y: 2))
        path.addLine(to: CGPoint(x: size - 8, y: size - 6))
        path.addLine(to: CGPoint(x: size / 2, y: size * 0.72))
        path.addLine(to: CGPoint(x: 8, y: size - 6))
        path.closeSubpath()
        context.addPath(path)
        context.setStrokeColor(UIColor.white.cgColor)
        context.setLineWidth(6)
        context.setLineJoin(.round)
        context.strokePath()
        context.addPath(path)
        context.setFillColor(mapColour("#E53935").cgColor)
        context.fillPath()
    }
}

private func renderLandings(_ style: MLNStyle, _ landings: [LandingPattern], _ items: [MissionItem], _ selected: Int?) {
    style.setGeoJson(LANDING_AREA_SOURCE, landingAreaFeatures(landings))
    style.setGeoJson(LANDING_LABEL_SOURCE, landingLabelFeatures(landings, selected, items: items))
    style.setGeoJson(LANDING_PATH_SOURCE, landingPathFeatures(landings))
    style.setGeoJson(LANDING_LOITER_SOURCE, landingLoiterFeatures(landings, items: items))
}

func clearAreaLift(_ topInset: CGFloat, _ bottomInset: CGFloat) -> CGFloat { (topInset - bottomInset) / 2 }

func outsideCentreInset(_ x: CGFloat, _ y: CGFloat, _ width: CGFloat, _ height: CGFloat, _ topInset: CGFloat, _ bottomInset: CGFloat) -> Bool {
    let side = width * CENTRE_INSET_FRACTION
    let top = topInset + height * CENTRE_INSET_FRACTION
    let bottom = height - bottomInset - height * CENTRE_INSET_FRACTION
    return width > 0 && height > 0 && (x < side || x > width - side || y < top || y > bottom)
}
