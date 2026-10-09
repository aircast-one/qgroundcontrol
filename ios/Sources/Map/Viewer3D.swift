import MapLibre
import SwiftUI

let VIEWER3D_VIEW = "view.viewer3d"
let VIEWER3D_PATH = "view.viewer3dPath"
let VIEWER3D_VEHICLE = "view.viewer3dVehicle"
private let V3D_FLOATING_SOURCE = "viewer3d-floating"
private let V3D_FLOATING_LAYER = "viewer3d-floating-layer"
private let V3D_VEHICLE_SOURCE = "viewer3d-vehicle"
private let V3D_VEHICLE_LAYER = "viewer3d-vehicle-layer"
private let V3D_BUILDING_SOURCE = "viewer3d-buildings"
private let V3D_BUILDING_LAYER = "viewer3d-buildings-layer"
private let RIBBON_MAX_PIECES = 64
private let METRES_PER_DEGREE = 111_320.0
private let RIBBON_WIDTH = 1.5
private let RIBBON_THICKNESS = 1.0
private let RIBBON_STEP = 8.0
private let MARKER_SIZE = 3.0
private let LABEL_LIFT = 3.0
private let ARM_LENGTH = 3.0
private let ARM_WIDTH = 0.5
private let ROTOR_SIZE = 1.6
private let HUB_SIZE = 1.4
private let FRONT_ARM_COLOUR = "#E53935"
private let REAR_ARM_COLOUR = "#ECEFF1"
private let ROTOR_COLOUR = "#37474F"
private let HUB_COLOUR = "#90A4AE"
private let SELECTED_MARKER_COLOUR = "#FFFF00"
private let MARKER_PICK_DP: Float = 24
private let SCENE_PITCH = 60.0
private let SCENE_ZOOM = 16.0
private let BUILDING_COLOUR = "#B0BEC5"
private let BUILDING_OPACITY = 0.85

struct Point3D: Equatable {
    var lon: Double
    var lat: Double
    var alt: Double
}

struct Slab {
    var corners: [(Double, Double)]
    var base: Double
    var top: Double
    var colour: String
}

private func radians(_ degrees: Double) -> Double { degrees * .pi / 180 }

private func point3d(_ array: JSON) -> Point3D? {
    let values = array.array.map { $0.double ?? .nan }
    guard values.count == 3, values.allSatisfy(\.isFinite) else { return nil }
    return Point3D(lon: values[0], lat: values[1], alt: values[2])
}

private func box(_ at: Point3D, _ size: Double, _ colour: String, _ height: Double? = nil) -> Slab {
    let tall = height ?? size
    let dLat = size / 2 / METRES_PER_DEGREE
    let dLon = dLat / cos(radians(at.lat))
    return Slab(
        corners: [(at.lon - dLon, at.lat - dLat), (at.lon + dLon, at.lat - dLat), (at.lon + dLon, at.lat + dLat), (at.lon - dLon, at.lat + dLat)],
        base: at.alt - tall / 2,
        top: at.alt + tall / 2,
        colour: colour
    )
}

func ribbon(_ from: Point3D, _ to: Point3D, _ colour: String, _ width: Double = RIBBON_WIDTH, _ thickness: Double = RIBBON_THICKNESS) -> [Slab] {
    let scale = cos(radians((from.lat + to.lat) / 2))
    let dx = (to.lon - from.lon) * METRES_PER_DEGREE * scale
    let dy = (to.lat - from.lat) * METRES_PER_DEGREE
    let length = hypot(dx, dy)
    if length < width / 2 {
        return [withChanges(box(from, width, colour, 0)) {
            $0.base = min(from.alt, to.alt) - thickness / 2
            $0.top = max(from.alt, to.alt) + thickness / 2
        }]
    }
    let pieces = min(max(Int((length / RIBBON_STEP).rounded(.up)), 1), RIBBON_MAX_PIECES)
    let climb = abs(to.alt - from.alt) / Double(pieces)
    let nx = -dy / length * width / 2
    let ny = dx / length * width / 2
    func lonLat(_ x: Double, _ y: Double) -> (Double, Double) {
        (from.lon + x / (METRES_PER_DEGREE * scale), from.lat + y / METRES_PER_DEGREE)
    }
    return (0..<pieces).map { piece in
        let t0 = Double(piece) / Double(pieces)
        let t1 = Double(piece + 1) / Double(pieces)
        let alt = from.alt + (to.alt - from.alt) * (t0 + t1) / 2
        let half = max(thickness, climb) / 2
        let (x0, y0, x1, y1) = (dx * t0, dy * t0, dx * t1, dy * t1)
        return Slab(
            corners: [lonLat(x0 + nx, y0 + ny), lonLat(x1 + nx, y1 + ny), lonLat(x1 - nx, y1 - ny), lonLat(x0 - nx, y0 - ny)],
            base: alt - half,
            top: alt + half,
            colour: colour
        )
    }
}

private func offset(_ at: Point3D, _ east: Double, _ north: Double) -> Point3D {
    Point3D(lon: at.lon + east / (METRES_PER_DEGREE * cos(radians(at.lat))), lat: at.lat + north / METRES_PER_DEGREE, alt: at.alt)
}

func quadFrame(_ at: Point3D, _ heading: Double) -> [Slab] {
    [45.0, -45.0, 135.0, -135.0].flatMap { arm -> [Slab] in
        let bearing = radians(heading + arm)
        let motor = offset(at, sin(bearing) * ARM_LENGTH, cos(bearing) * ARM_LENGTH)
        let colour = abs(arm) < 90 ? FRONT_ARM_COLOUR : REAR_ARM_COLOUR
        return ribbon(at, motor, colour, ARM_WIDTH, ARM_WIDTH)
            + [box(Point3D(lon: motor.lon, lat: motor.lat, alt: at.alt + ARM_WIDTH), ROTOR_SIZE, ROTOR_COLOUR, ARM_WIDTH / 2)]
    } + [box(at, HUB_SIZE, HUB_COLOUR, HUB_SIZE / 2)]
}

struct PathMarker: Equatable {
    var mission: Int
    var at: Point3D?
}

func pathMarkers(_ view: JSON?) -> [PathMarker] {
    (view?["markers"].arrayOrNil ?? []).map { PathMarker(mission: $0["mission"].int(0), at: point3d($0["at"])) }
}

func selectedAfterTap(_ selected: Set<Int>, _ markers: [PathMarker], _ hit: Int?) -> Set<Int> {
    guard let picked = hit else { return [] }
    return selected.filter { markers.indices.contains($0) ? markers[$0].mission != markers[picked].mission : true }.union([picked])
}

private func missionMarkers(_ markers: [PathMarker], _ mission: Int) -> [(offset: Int, element: PathMarker)] {
    markers.enumerated().filter { $0.element.mission == mission }.map { ($0.offset, $0.element) }
}

func selectionKept(_ selected: Set<Int>, _ before: [PathMarker], _ after: [PathMarker]) -> Set<Int> {
    Set(selected.compactMap { index -> Int? in
        guard before.indices.contains(index) else { return nil }
        let mission = before[index].mission
        let was = missionMarkers(before, mission)
        let now = missionMarkers(after, mission)
        guard now.map(\.element) == was.map(\.element), let at = was.firstIndex(where: { $0.offset == index }), now.indices.contains(at) else { return nil }
        return now[at].offset
    })
}

func pickedMarker(_ tap: (Float, Float), _ onScreen: [(Float, Float)?], _ radius: Float) -> Int? {
    onScreen.enumerated()
        .compactMap { index, at in at.map { (index, hypotf($0.0 - tap.0, $0.1 - tap.1)) } }
        .filter { $0.1 <= radius }
        .min { $0.1 < $1.1 }?
        .0
}

func pathSlabs(_ view: JSON?, _ selected: Set<Int> = []) -> [Slab] {
    let ribbons = (view?["segments"].objects ?? []).flatMap { segment -> [Slab] in
        guard let from = point3d(segment["from"]), let to = point3d(segment["to"]) else { return [] }
        return ribbon(from, to, segment["colour"].string)
    }
    let boxes = (view?["markers"].arrayOrNil ?? []).enumerated().compactMap { index, marker -> Slab? in
        guard marker.object != nil else { return nil }
        return point3d(marker["at"]).map { box($0, MARKER_SIZE, selected.contains(index) ? SELECTED_MARKER_COLOUR : marker["colour"].string) }
    }
    return ribbons + boxes
}

struct Label3D: Equatable {
    var at: Point3D
    var text: String
}

func pathLabels(_ view: JSON?) -> [Label3D] {
    (view?["markers"].objects ?? []).compactMap { marker in
        point3d(marker["at"]).map { Label3D(at: Point3D(lon: $0.lon, lat: $0.lat, alt: $0.alt + LABEL_LIFT), text: marker["label"].string) }
    }
    .filter { !$0.text.isBlank }
}

func lifted(_ ground: (Float, Float), _ pixelsPerMetre: Double, _ tilt: Double, _ height: Double) -> (Float, Float) {
    (ground.0, Float(Double(ground.1) - max(height, 0) * pixelsPerMetre * sin(radians(tilt))))
}

private func onScreen(_ map: MLNMapView, _ at: Point3D) -> (Float, Float) {
    let across = radians(map.direction + 90)
    let ground = map.convert(CLLocationCoordinate2D(latitude: at.lat, longitude: at.lon), toPointTo: map)
    let side = offset(at, sin(across), cos(across))
    let beside = map.convert(CLLocationCoordinate2D(latitude: side.lat, longitude: side.lon), toPointTo: map)
    let pixelsPerMetre = hypot(Double(beside.x - ground.x), Double(beside.y - ground.y))
    return lifted((Float(ground.x), Float(ground.y)), pixelsPerMetre, Double(map.camera.pitch), at.alt)
}

func vehicleSlabs(_ view: JSON?) -> [Slab] {
    (view?["vehicles"].objects ?? []).flatMap { vehicle in
        point3d(vehicle["at"]).map { quadFrame($0, vehicle["heading"].double(0)) } ?? []
    }
}

func slabFeatures(_ slabs: [Slab]) -> FeatureCollection {
    featureCollection(slabs.map { slab in
        polygonFeature(
            (slab.corners + slab.corners.prefix(1)).map { TrackPoint(latitude: $0.1, longitude: $0.0) },
            attributes: ["base": max(slab.base, 0), "top": max(slab.top, 0.1), "colour": slab.colour]
        )
    })
}

struct Building3D {
    var outer: [[(Double, Double)]]
    var inner: [[(Double, Double)]]
    var height: Double
}

struct Scene3D {
    var available: Bool
    var reason: String
    var buildings: [Building3D]
    var centre: (Double, Double)?
}

private func lonLats(_ array: JSON) -> [(Double, Double)] {
    array.array.compactMap { $0.arrayOrNil == nil ? nil : ($0[0].double(.nan), $0[1].double(.nan)) }
}

private func rings(_ array: JSON) -> [[(Double, Double)]] {
    array.array.filter { $0.arrayOrNil != nil }.map(lonLats).filter { $0.count > 2 }
}

func signedArea(_ ring: [(Double, Double)]) -> Double {
    zip(ring, ring.dropFirst() + ring.prefix(1)).map { a, b in a.0 * b.1 - b.0 * a.1 }.reduce(0, +) / 2
}

func wound(_ ring: [(Double, Double)], _ counterClockwise: Bool) -> [(Double, Double)] {
    guard let first = ring.first, let last = ring.last else { return ring }
    let closed = first == last ? ring : ring + [first]
    return (signedArea(closed) > 0) == counterClockwise ? closed : closed.reversed()
}

private func boundsCentre(_ bounds: JSON) -> (Double, Double)? {
    let edges = ["west", "east", "south", "north"].compactMap { bounds[$0].double }
    guard edges.count == 4, edges.allSatisfy(\.isFinite) else { return nil }
    return ((edges[0] + edges[1]) / 2, (edges[2] + edges[3]) / 2)
}

func scene3d(_ view: JSON?) -> Scene3D {
    let bounds = view?["bounds"] ?? .null
    return Scene3D(
        available: view?["available"].bool == true,
        reason: view?["reason"].string ?? "",
        buildings: (view?["buildings"].objects ?? [])
            .map { Building3D(outer: rings($0["outer"]), inner: rings($0["inner"]), height: $0["height"].double(0)) }
            .filter { !$0.outer.isEmpty && $0.height > 0 },
        centre: boundsCentre(bounds)
    )
}

private func polygon(_ ring: [(Double, Double)], _ holes: [MLNPolygon]? = nil) -> MLNPolygonFeature {
    let coordinates = ring.map { CLLocationCoordinate2D(latitude: $0.1, longitude: $0.0) }
    return MLNPolygonFeature(coordinates: coordinates, count: UInt(coordinates.count), interiorPolygons: holes)
}

func buildingFeatures(_ buildings: [Building3D]) -> FeatureCollection {
    featureCollection(buildings.map { building -> Feature in
        let outers = building.outer.map { wound($0, true) }
        let holes = building.inner.map { polygon(wound($0, false)) }
        let feature: Feature = outers.count == 1
            ? polygon(outers[0], holes)
            : MLNMultiPolygonFeature(polygons: outers.map { polygon($0) })
        feature.attributes = ["height": building.height]
        return feature
    })
}

private func installScene(_ style: MLNStyle) {
    let buildings = geoJsonSource(V3D_BUILDING_SOURCE)
    style.addSource(buildings)
    let layer = MLNFillExtrusionStyleLayer(identifier: V3D_BUILDING_LAYER, source: buildings)
    layer.fillExtrusionColor = styleConstant(mapColour(BUILDING_COLOUR))
    layer.fillExtrusionHeight = styleGet("height")
    layer.fillExtrusionBase = styleConstant(0)
    layer.fillExtrusionOpacity = styleConstant(BUILDING_OPACITY)
    style.addLayer(layer)
    addSlabLayer(style, V3D_FLOATING_SOURCE, V3D_FLOATING_LAYER)
    addSlabLayer(style, V3D_VEHICLE_SOURCE, V3D_VEHICLE_LAYER)
}

private func addSlabLayer(_ style: MLNStyle, _ source: String, _ layer: String) {
    let shapes = geoJsonSource(source)
    style.addSource(shapes)
    let slabs = MLNFillExtrusionStyleLayer(identifier: layer, source: shapes)
    slabs.fillExtrusionColor = NSExpression(format: "CAST(colour, 'UIColor')")
    slabs.fillExtrusionBase = styleGet("base")
    slabs.fillExtrusionHeight = styleGet("top")
    style.addLayer(slabs)
}

private struct PlacedLabel: Identifiable, Equatable {
    let id: Int
    let at: CGPoint
    let text: String
}

struct Viewer3DPane: View {
    @MapPath(VIEWER3D_VIEW) private var viewJson
    @MapPath(VEHICLES_VIEW) private var vehiclesJson
    @MapPath(VIEWER3D_PATH) private var pathJson
    @MapPath(VIEWER3D_VEHICLE) private var vehicleJson
    @State private var selectedMarkers: Set<Int> = []
    @State private var selectedAmong: [PathMarker] = []
    @State private var placed: [PlacedLabel] = []
    @Environment(\.theme) private var theme

    var body: some View {
        let scene = scene3d(viewJson)
        let vehicle = vehicleChoices(vehiclesJson).choices.first { $0.active && isPlottable($0.latitude, $0.longitude) }
        let markerList = pathMarkers(pathJson)
        ZStack(alignment: .topLeading) {
            Viewer3DMap(
                sceneKey: viewJson,
                buildings: scene.buildings,
                pathKey: PathKey(path: pathJson, selected: selectedMarkers),
                slabs: pathSlabs(pathJson, selectedMarkers),
                vehicleKey: vehicleJson,
                frame: vehicleSlabs(vehicleJson),
                centre: scene.centre.map { CLLocationCoordinate2D(latitude: $0.1, longitude: $0.0) },
                vehicle: vehicle.map { CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude) },
                markers: markerList,
                labels: pathLabels(pathJson),
                onTap: { hit in selectedMarkers = selectedAfterTap(selectedMarkers, markerList, hit) },
                placed: $placed
            )
            ForEach(placed) { label in
                Text(label.text)
                    .font(.labelLarge)
                    .foregroundStyle(Color.black)
                    .fixedSize()
                    .offset(x: label.at.x, y: label.at.y)
            }
            if !scene.available && !scene.reason.isBlank {
                Text(scene.reason)
                    .font(.bodyMedium)
                    .padding(16)
                    .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
                    .padding(24)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
        }
        .clipped()
        .onChange(of: markerList, initial: true) {
            selectedMarkers = selectionKept(selectedMarkers, selectedAmong, markerList)
            selectedAmong = markerList
        }
    }
}

private struct PathKey: Equatable {
    let path: JSON?
    let selected: Set<Int>
}

private let ORNAMENT_MARGIN: CGFloat = 8

private struct Viewer3DMap: UIViewRepresentable {
    let sceneKey: JSON?
    let buildings: [Building3D]
    let pathKey: PathKey
    let slabs: [Slab]
    let vehicleKey: JSON?
    let frame: [Slab]
    let centre: CLLocationCoordinate2D?
    let vehicle: CLLocationCoordinate2D?
    let markers: [PathMarker]
    let labels: [Label3D]
    let onTap: (Int?) -> Void
    @Binding var placed: [PlacedLabel]

    func makeCoordinator() -> Coordinator { Coordinator(self) }

    func makeUIView(context: Context) -> MLNMapView {
        let view = MLNMapView(frame: .zero)
        view.delegate = context.coordinator
        view.isRotateEnabled = true
        view.isPitchEnabled = true
        let tap = UITapGestureRecognizer(target: context.coordinator, action: #selector(Coordinator.tapped(_:)))
        tap.delegate = context.coordinator
        view.addGestureRecognizer(tap)
        offMain {
            let url = qgcStyleURL(currentMapType())
            onMain { view.styleURL = url }
        }
        return view
    }

    func updateUIView(_ view: MLNMapView, context: Context) {
        context.coordinator.parent = self
        context.coordinator.apply(view)
        let slack = max(0, (MINIMUM_TOUCH_TARGET - view.attributionButton.intrinsicContentSize.width) / 2)
        view.attributionButtonMargins = CGPoint(x: ORNAMENT_MARGIN - slack, y: ORNAMENT_MARGIN - slack)
        view.updateConstraintsIfNeeded()
        view.attributionButton.constraints
            .filter { $0.firstAttribute == .width || $0.firstAttribute == .height }
            .forEach { $0.constant = MINIMUM_TOUCH_TARGET }
    }

    final class Coordinator: NSObject, MLNMapViewDelegate, UIGestureRecognizerDelegate {
        var parent: Viewer3DMap
        private var style: MLNStyle?
        private var framedOn: CLLocationCoordinate2D?
        private var shownScene: JSON??
        private var shownPath: PathKey?
        private var shownVehicle: JSON??

        init(_ parent: Viewer3DMap) { self.parent = parent }

        func mapView(_ mapView: MLNMapView, didFinishLoading style: MLNStyle) {
            style.transition = NO_FADES
            installScene(style)
            self.style = style
            shownScene = nil
            shownPath = nil
            shownVehicle = nil
            apply(mapView)
        }

        func mapViewRegionIsChanging(_ mapView: MLNMapView) { place(mapView) }

        func mapView(_ mapView: MLNMapView, regionDidChangeAnimated animated: Bool) { place(mapView) }

        func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
            true
        }

        @objc func tapped(_ tap: UITapGestureRecognizer) {
            guard let map = tap.view as? MLNMapView else { return }
            let point = tap.location(in: map)
            let screen = parent.markers.map { marker in marker.at.map { onScreen(map, $0) } }
            parent.onTap(pickedMarker((Float(point.x), Float(point.y)), screen, MARKER_PICK_DP))
        }

        func apply(_ map: MLNMapView) {
            frame(map)
            guard let style else { return }
            if shownScene != .some(parent.sceneKey) {
                shownScene = .some(parent.sceneKey)
                style.setGeoJson(V3D_BUILDING_SOURCE, buildingFeatures(parent.buildings))
            }
            if shownPath != parent.pathKey {
                shownPath = parent.pathKey
                style.setGeoJson(V3D_FLOATING_SOURCE, slabFeatures(parent.slabs))
            }
            if shownVehicle != .some(parent.vehicleKey) {
                shownVehicle = .some(parent.vehicleKey)
                style.setGeoJson(V3D_VEHICLE_SOURCE, slabFeatures(parent.frame))
            }
            place(map)
        }

        private func frame(_ map: MLNMapView) {
            guard let target = parent.centre ?? parent.vehicle else { return }
            let same = framedOn.map { $0.latitude == target.latitude && $0.longitude == target.longitude } ?? false
            guard !same, framedOn == nil || parent.centre != nil else { return }
            framedOn = target
            map.setCenter(target, zoomLevel: SCENE_ZOOM, animated: false)
            let camera = map.camera
            camera.pitch = SCENE_PITCH
            map.setCamera(camera, animated: false)
        }

        private func place(_ map: MLNMapView) {
            guard style != nil else { return }
            let next = parent.labels.enumerated().map { index, label in
                let (x, y) = onScreen(map, label.at)
                return PlacedLabel(id: index, at: CGPoint(x: CGFloat(x), y: CGFloat(y)), text: label.text)
            }
            guard next != parent.placed else { return }
            DispatchQueue.main.async { [parent] in parent.placed = next }
        }
    }
}
