import AppKit
import MapKit
import SwiftUI

final class MissionAnnotation: NSObject, MKAnnotation {
    @objc dynamic var coordinate: CLLocationCoordinate2D
    let title: String?
    let subtitle: String?
    let sequence: Int
    let isSelected: Bool
    let isLaunch: Bool
    let canMove: Bool

    init(item: MissionItem, latitude: Double, longitude: Double) {
        coordinate = CLLocationCoordinate2D(latitude: latitude, longitude: longitude)
        title = "\(item.sequence). \(item.command)"
        subtitle = item.mapSubtitle
        sequence = item.sequence
        isSelected = item.isSelected
        isLaunch = item.isLaunch
        canMove = item.canMove
    }
}

final class RallyAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String?
    let subtitle: String?

    init(point: RallyPointRow) {
        coordinate = CLLocationCoordinate2D(latitude: point.latitude ?? 0,
                                            longitude: point.longitude ?? 0)
        title = "Rally \(point.id + 1)"
        subtitle = point.altitudeText == "\u{2014}" ? nil : point.altitudeText
    }
}

final class FencePolygon: MKPolygon {
    var inclusion = false
}

final class SurveyPolygon: MKPolygon {}

final class CorridorPolyline: MKPolyline {}
final class TransectPolyline: MKPolyline {}
final class TrackPolyline: MKPolyline {}

final class FenceCircle: MKCircle {
    var inclusion = false
}

final class FirmwareFenceCircle: MKCircle {}

final class OrbitCircle: MKCircle {}

final class GotoAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String? = "Flying here"

    init(point: GeoPoint) {
        coordinate = CLLocationCoordinate2D(latitude: point.latitude, longitude: point.longitude)
    }
}

final class VertexAnnotation: NSObject, MKAnnotation {
    @objc dynamic var coordinate: CLLocationCoordinate2D
    let polygon: Int
    let index: Int
    let midpoint: Bool
    let removable: Bool
    let hint: String
    var title: String? { midpoint ? "Add a corner" : "Corner \(index + 1)" }
    var subtitle: String? { midpoint || removable ? nil : hint }

    init(polygon: Int, index: Int, point: GeoPoint, midpoint: Bool,
         removable: Bool = false, hint: String = "") {
        coordinate = CLLocationCoordinate2D(latitude: point.latitude, longitude: point.longitude)
        self.polygon = polygon
        self.index = index
        self.midpoint = midpoint
        self.removable = removable
        self.hint = hint
    }
}

final class ClickAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String? = "This spot"

    init(point: GeoPoint) {
        coordinate = CLLocationCoordinate2D(latitude: point.latitude, longitude: point.longitude)
    }
}

final class RoiAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String? = "Looking here"

    init(point: GeoPoint) {
        coordinate = CLLocationCoordinate2D(latitude: point.latitude, longitude: point.longitude)
    }
}

final class VehicleAnnotation: NSObject, MKAnnotation {
    let coordinate: CLLocationCoordinate2D
    let title: String? = "Vehicle"
    let rotation: Double
    let hasHeading: Bool

    init(marker: VehicleMarker) {
        coordinate = CLLocationCoordinate2D(latitude: marker.latitude, longitude: marker.longitude)
        rotation = marker.rotationRadians
        hasHeading = marker.hasHeading
    }
}

struct MissionMap: NSViewRepresentable {
    let owner: String
    let items: [MissionItem]
    let vehicle: VehicleMarker?
    let shapes: [FenceShape]
    let rallyPoints: [RallyPointRow]
    let padding: NSEdgeInsets
    let select: (Int) -> Void
    let adding: Bool
    let add: (Double, Double) -> Void
    let move: (Int, Double, Double) -> Void
    var firmwareFence: FirmwareFence?
    var secondary: ((Double, Double, CGPoint) -> Void)?
    var surveys: [[GeoPoint]] = []
    var corridors: [[GeoPoint]] = []
    var transects: [[GeoPoint]] = []
    var focus: MapFrame?
    var polygons: [EditablePolygon] = []
    var moveVertex: (Int, Int, Double, Double) -> Void = { _, _, _, _ in }
    var removeVertexAt: (Int, Int) -> Void = { _, _ in }
    var splitSegment: (Int, Int) -> Void = { _, _ in }
    var overlays = FlyOverlays.none
    var track: [GeoPoint] = []
    var follow = false
    var tracking = false

    func makeNSView(context: Context) -> MKMapView {
        let map = MKMapView()
        map.delegate = context.coordinator

        let mapType = CachedTileOverlay.currentMapType()
        if !mapType.isEmpty {
            context.coordinator.overlay = CachedTileOverlay(mapType: mapType)
            map.addOverlay(context.coordinator.overlay!, level: .aboveRoads)
        }
        context.coordinator.add = add
        context.coordinator.move = move
        context.coordinator.secondary = secondary
        context.coordinator.moveVertex = moveVertex
        context.coordinator.removeVertexAt = removeVertexAt
        context.coordinator.splitSegment = splitSegment
        map.showsCompass = true
        map.showsScale = false
        map.isPitchEnabled = false
        return map
    }

    func updateNSView(_ map: MKMapView, context: Context) {
        context.coordinator.add = add
        context.coordinator.move = move
        context.coordinator.secondary = secondary
        context.coordinator.moveVertex = moveVertex
        context.coordinator.removeVertexAt = removeVertexAt
        context.coordinator.splitSegment = splitSegment
        context.coordinator.arm(adding, on: map)
        context.coordinator.armSecondary(on: map)
        map.removeAnnotations(map.annotations)
        map.overlays.filter { !($0 is CachedTileOverlay) }.forEach(map.removeOverlay)

        let placed = items.compactMap { item -> MissionAnnotation? in
            guard item.hasPosition, let latitude = item.latitude, let longitude = item.longitude else {
                return nil
            }
            return MissionAnnotation(item: item, latitude: latitude, longitude: longitude)
        }
        map.addAnnotations(placed)

        if let vehicle {
            map.addAnnotation(VehicleAnnotation(marker: vehicle))
        }

        polygons.enumerated().forEach { polygonIndex, polygon in
            polygon.points.enumerated().forEach { index, point in
                map.addAnnotation(VertexAnnotation(polygon: polygonIndex, index: index,
                                                   point: point, midpoint: false,
                                                   removable: PolygonEdit.removes(index, in: polygon),
                                                   hint: PolygonEdit.removalHint(polygon)))
            }
            polygon.midpoints.enumerated().forEach { index, point in
                map.addAnnotation(VertexAnnotation(polygon: polygonIndex, index: index,
                                                   point: point, midpoint: true))
            }
        }

        if let going = overlays.goingTo {
            map.addAnnotation(GotoAnnotation(point: going))
        }

        if let clicked = overlays.clickedAt {
            map.addAnnotation(ClickAnnotation(point: clicked))
        }

        if overlays.showsRoi, let looking = overlays.roiAt {
            map.addAnnotation(RoiAnnotation(point: looking))
        }

        if overlays.showsOrbit, let centre = overlays.orbitCentre {
            map.addOverlay(OrbitCircle(center: CLLocationCoordinate2D(latitude: centre.latitude,
                                                                     longitude: centre.longitude),
                                       radius: overlays.orbitRadius),
                           level: .aboveLabels)
        }

        let rally = rallyPoints.filter { $0.latitude != nil && $0.longitude != nil }
        map.addAnnotations(rally.map(RallyAnnotation.init(point:)))
        shapes.compactMap(MissionMap.overlay(for:)).forEach { map.addOverlay($0, level: .aboveLabels) }
        firmwareFence.flatMap(MissionMap.overlay(for:))
            .map { map.addOverlay($0, level: .aboveLabels) }

        surveys.filter { $0.count >= 3 }.forEach { area in
            var corners = area.map { CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude) }
            map.addOverlay(SurveyPolygon(coordinates: &corners, count: corners.count), level: .aboveLabels)
        }

        corridors.filter { $0.count >= 2 }.forEach { path in
            var points = path.map { CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude) }
            map.addOverlay(CorridorPolyline(coordinates: &points, count: points.count),
                           level: .aboveLabels)
        }

        transects.forEach { line in
            var points = line.map { CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude) }
            map.addOverlay(TransectPolyline(coordinates: &points, count: points.count),
                           level: .aboveLabels)
        }

        // The legs, not the markers. A region of interest earns a pin and no leg, and an item
        // after a return to launch is uploaded and never reached.
        let route = MissionItem.routePoints(items).map {
            CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)
        }
        if route.count > 1 {
            var coordinates = route
            map.addOverlay(MKPolyline(coordinates: &coordinates, count: coordinates.count),
                           level: .aboveLabels)
        }

        if track.count > 1 {
            var flown = track.map {
                CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)
            }
            map.addOverlay(TrackPolyline(coordinates: &flown, count: flown.count),
                           level: .aboveLabels)
        }

        MissionMap.lastRender[owner] = [
            "items": items.count,
            "placed": placed.count,
            "routePoints": route.count,
            "annotations": map.annotations.count,
            "rally": rally.count,
            "fenceOverlays": map.overlays.filter { $0 is FencePolygon || $0 is FenceCircle }.count,
            "surveyOverlays": map.overlays.filter { $0 is SurveyPolygon }.count,
            "corridorOverlays": map.overlays.filter { $0 is CorridorPolyline }.count,
            "overlays": map.overlays.count,
            "framed": context.coordinator.lastFrame != nil,
            "centre": ["lat": map.centerCoordinate.latitude, "lon": map.centerCoordinate.longitude],
            "spanLat": map.region.span.latitudeDelta,
            "spanLon": map.region.span.longitudeDelta,
            "tileOverlay": context.coordinator.overlay != nil,
            "mapType": CachedTileOverlay.currentMapType(),
            "size": ["w": Double(map.bounds.width), "h": Double(map.bounds.height)],
        ]

        let framable = placed.map(\.coordinate)
            + rally.compactMap { point in
                point.latitude.flatMap { latitude in
                    point.longitude.map { CLLocationCoordinate2D(latitude: latitude, longitude: $0) }
                }
            }
            + shapes.flatMap(\.framingPoints).map {
                CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)
            }
            + surveys.flatMap { $0 }.map {
                CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)
            }

        let anchored = framable.isEmpty
            ? vehicle.map { [CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)] } ?? []
            : framable

        if tracking, let marker = vehicle {
            let target = CLLocationCoordinate2D(latitude: marker.latitude,
                                                longitude: marker.longitude)
            let inset = MapInsets(top: padding.top, left: padding.left,
                                  bottom: padding.bottom, right: padding.right)
            let screen = map.convert(target, toPointTo: map)
            let point = MapPoint(x: screen.x, y: screen.y)
            let width = map.bounds.width
            let height = map.bounds.height

            if MapFollow.follows(setting: follow, tracking: true) {
                map.setCenter(target, animated: false)
                MissionMap.recordCentre(owner: owner, map: map)
                return
            }
            if MapFollow.nudges(setting: follow, tracking: true),
               MapFollow.needsRecentre(vehicle: point, width: width, height: height,
                                       insets: inset),
               let shift = MapFollow.offset(width: width, height: height, insets: inset) {
                let moved = CGPoint(x: screen.x + shift.x, y: screen.y + shift.y)
                map.setCenter(map.convert(moved, toCoordinateFrom: map), animated: false)
                MissionMap.recordCentre(owner: owner, map: map)
                return
            }
        }

        if let focus, focus != context.coordinator.lastFocus {
            context.coordinator.lastFocus = focus
            context.coordinator.lastFrame = focus
            map.setRegion(MissionMap.region(focus, padding: padding, in: map.bounds.size),
                          animated: false)
            MissionMap.recordCentre(owner: owner, map: map)
            return
        }

        let frame = MapFrame(latitudes: anchored.map(\.latitude),
                             longitudes: anchored.map(\.longitude))

        if !anchored.isEmpty, frame != context.coordinator.lastFrame {
            guard map.bounds.width > 0 else {
                DispatchQueue.main.async { [weak map] in
                    guard let map, map.bounds.width > 0,
                          frame != context.coordinator.lastFrame else { return }
                    context.coordinator.lastFrame = frame
                    map.setRegion(MissionMap.region(frame, padding: padding, in: map.bounds.size), animated: false)
                }
                return
            }
            context.coordinator.lastFrame = frame
            map.setRegion(MissionMap.region(frame, padding: padding, in: map.bounds.size), animated: false)
        }

        MissionMap.recordCentre(owner: owner, map: map)
    }

    static func scale(of map: MKMapView) -> MapScaleBar {
        let width = map.bounds.width
        guard width > MissionMap.scalePixels else { return .none }
        let left = map.convert(CGPoint(x: 0, y: map.bounds.midY), toCoordinateFrom: map)
        let right = map.convert(CGPoint(x: MissionMap.scalePixels, y: map.bounds.midY),
                                toCoordinateFrom: map)
        let metres = CLLocation(latitude: left.latitude, longitude: left.longitude)
            .distance(from: CLLocation(latitude: right.latitude, longitude: right.longitude))
        guard let across = MapScaleBar.across(metres.rounded()) else { return .none }
        return MapScaleBar(Bridge.group("view.mapScale(\(across))")) ?? .none
    }

    static let scalePixels = 100.0

    static func recordCentre(owner: String, map: MKMapView) {
        lastRender[owner]?["centre"] =
            ["lat": map.centerCoordinate.latitude, "lon": map.centerCoordinate.longitude]
        lastRender[owner]?["spanLat"] = map.region.span.latitudeDelta
        lastRender[owner]?["spanLon"] = map.region.span.longitudeDelta
        let bar = scale(of: map)
        lastRender[owner]?["scale"] = bar.text
        lastScale[owner] = bar
    }

    static func overlay(for enforced: FirmwareFence) -> MKOverlay? {
        guard let centre = enforced.centre else { return nil }
        return FirmwareFenceCircle(
            center: CLLocationCoordinate2D(latitude: centre.latitude, longitude: centre.longitude),
            radius: enforced.radiusMetres)
    }

    static func overlay(for shape: FenceShape) -> MKOverlay? {
        if shape.isCircle {
            guard let centre = shape.centre, let radius = shape.drawnRadius else { return nil }
            let circle = FenceCircle(
                center: CLLocationCoordinate2D(latitude: centre.latitude,
                                               longitude: centre.longitude),
                radius: radius)
            circle.inclusion = shape.inclusion
            return circle
        }

        guard shape.vertices.count >= 3 else { return nil }
        var coordinates = shape.vertices.map {
            CLLocationCoordinate2D(latitude: $0.latitude, longitude: $0.longitude)
        }
        let polygon = FencePolygon(coordinates: &coordinates, count: coordinates.count)
        polygon.inclusion = shape.inclusion
        return polygon
    }

    static func region(_ frame: MapFrame, padding: NSEdgeInsets, in size: CGSize) -> MKCoordinateRegion {
        let width = max(size.width, 1)
        let height = max(size.height, 1)
        let visibleWidth = max(width - padding.left - padding.right, 1)
        let visibleHeight = max(height - padding.top - padding.bottom, 1)

        let longitudeDelta = frame.longitudeDelta * width / visibleWidth
        let latitudeDelta = frame.latitudeDelta * height / visibleHeight

        return MKCoordinateRegion(
            center: CLLocationCoordinate2D(
                latitude: frame.centreLatitude
                    + latitudeDelta * (padding.top - padding.bottom) / (2 * height),
                longitude: frame.centreLongitude
                    + longitudeDelta * (padding.right - padding.left) / (2 * width)),
            span: MKCoordinateSpan(latitudeDelta: min(latitudeDelta, 90),
                                   longitudeDelta: min(longitudeDelta, 180)))
    }

    static func rect(_ frame: MapFrame) -> MKMapRect {
        let north = MKMapPoint(CLLocationCoordinate2D(
            latitude: frame.centreLatitude + frame.latitudeDelta / 2,
            longitude: frame.centreLongitude - frame.longitudeDelta / 2))
        let south = MKMapPoint(CLLocationCoordinate2D(
            latitude: frame.centreLatitude - frame.latitudeDelta / 2,
            longitude: frame.centreLongitude + frame.longitudeDelta / 2))
        return MKMapRect(x: min(north.x, south.x), y: min(north.y, south.y),
                         width: abs(south.x - north.x), height: abs(south.y - north.y))
    }


    func makeCoordinator() -> Coordinator { Coordinator(select: select) }

    static var lastRender: [String: [String: Any]] = [:]
    static var lastScale: [String: MapScaleBar] = [:]
    static var rendererCalls = 0
    static var rendererKinds: Set<String> = []

    final class Coordinator: NSObject, MKMapViewDelegate {
        let select: (Int) -> Void
        var add: (Double, Double) -> Void = { _, _ in }
        var move: (Int, Double, Double) -> Void = { _, _, _ in }
        var secondary: ((Double, Double, CGPoint) -> Void)?
        private var secondaryClick: NSClickGestureRecognizer?
        var lastFrame: MapFrame?
        var lastFocus: MapFrame?
        private var placer: NSClickGestureRecognizer?

        init(select: @escaping (Int) -> Void) {
            self.select = select
        }

        deinit {
            if placer != nil {
                NSCursor.pop()
            }
        }

        func armSecondary(on map: MKMapView) {
            guard secondary != nil, secondaryClick == nil else { return }
            let recognizer = NSClickGestureRecognizer(target: self,
                                                      action: #selector(secondaryTap(_:)))
            recognizer.buttonMask = 0x2
            map.addGestureRecognizer(recognizer)
            secondaryClick = recognizer
        }

        @objc private func secondaryTap(_ recognizer: NSClickGestureRecognizer) {
            guard let map = recognizer.view as? MKMapView else { return }
            let point = recognizer.location(in: map)
            let coordinate = map.convert(point, toCoordinateFrom: map)
            secondary?(coordinate.latitude, coordinate.longitude, point)
        }

        func arm(_ adding: Bool, on map: MKMapView) {
            if adding, placer == nil {
                let recognizer = NSClickGestureRecognizer(target: self, action: #selector(placeWaypoint(_:)))
                map.addGestureRecognizer(recognizer)
                placer = recognizer
                NSCursor.crosshair.push()
            } else if !adding, let recognizer = placer {
                map.removeGestureRecognizer(recognizer)
                placer = nil
                NSCursor.pop()
            }
        }

        @objc private func placeWaypoint(_ recognizer: NSClickGestureRecognizer) {
            guard let map = recognizer.view as? MKMapView else { return }
            let point = recognizer.location(in: map)
            let coordinate = map.convert(point, toCoordinateFrom: map)
            add(coordinate.latitude, coordinate.longitude)
        }

        func mapView(_ mapView: MKMapView, annotationView view: MKAnnotationView,
                     didChange newState: MKAnnotationView.DragState,
                     fromOldState oldState: MKAnnotationView.DragState) {
            guard newState == .ending else { return }
            if let vertex = view.annotation as? VertexAnnotation, !vertex.midpoint {
                moveVertex(vertex.polygon, vertex.index,
                           vertex.coordinate.latitude, vertex.coordinate.longitude)
                return
            }
            guard let item = view.annotation as? MissionAnnotation else { return }
            move(item.sequence, item.coordinate.latitude, item.coordinate.longitude)
        }

        func mapView(_ mapView: MKMapView, didSelect view: MKAnnotationView) {
            if let vertex = view.annotation as? VertexAnnotation {
                // A midpoint is the split gesture itself, so it never rests as a selection; a
                // corner keeps its callout open, which is where the head offers Remove.
                if vertex.midpoint {
                    splitSegment(vertex.polygon, vertex.index)
                    mapView.deselectAnnotation(vertex, animated: false)
                }
                return
            }
            guard let item = view.annotation as? MissionAnnotation else { return }
            select(item.sequence)
        }

        static func removeButton(hint: String) -> NSButton {
            let button = NSButton(image: NSImage(systemSymbolName: "trash",
                                                 accessibilityDescription: hint) ?? NSImage(),
                                  target: nil, action: nil)
            button.bezelStyle = .accessoryBarAction
            button.toolTip = hint
            button.frame = NSRect(x: 0, y: 0, width: 26, height: 20)
            return button
        }

        func mapView(_ mapView: MKMapView, annotationView view: MKAnnotationView,
                     calloutAccessoryControlTapped control: NSControl) {
            guard let vertex = view.annotation as? VertexAnnotation, vertex.removable else { return }
            mapView.deselectAnnotation(vertex, animated: false)
            removeVertexAt(vertex.polygon, vertex.index)
        }

        var moveVertex: (Int, Int, Double, Double) -> Void = { _, _, _, _ in }
        var removeVertexAt: (Int, Int) -> Void = { _, _ in }
        var splitSegment: (Int, Int) -> Void = { _, _ in }

        static let clickRing = Coordinator.ring(18)
        static let vertexDot = Coordinator.dot(NSColor.controlAccentColor, 12)
        static let midpointDot = Coordinator.dot(NSColor.white.withAlphaComponent(0.85), 9)

        static func ring(_ size: CGFloat) -> NSImage {
            let image = NSImage(size: NSSize(width: size, height: size))
            image.lockFocus()
            NSColor.black.withAlphaComponent(0.6).setStroke()
            let outer = NSBezierPath(ovalIn: NSRect(x: 2, y: 2, width: size - 4, height: size - 4))
            outer.lineWidth = 4
            outer.stroke()
            NSColor.white.setStroke()
            let inner = NSBezierPath(ovalIn: NSRect(x: 2, y: 2, width: size - 4, height: size - 4))
            inner.lineWidth = 2
            inner.stroke()
            NSColor.white.setFill()
            let centre = size / 2
            NSBezierPath(ovalIn: NSRect(x: centre - 2, y: centre - 2, width: 4, height: 4)).fill()
            image.unlockFocus()
            return image
        }

        static func dot(_ colour: NSColor, _ size: CGFloat) -> NSImage {
            let image = NSImage(size: NSSize(width: size, height: size))
            image.lockFocus()
            colour.setFill()
            NSBezierPath(ovalIn: NSRect(x: 0, y: 0, width: size, height: size)).fill()
            NSColor.black.withAlphaComponent(0.4).setStroke()
            let ring = NSBezierPath(ovalIn: NSRect(x: 0.5, y: 0.5, width: size - 1, height: size - 1))
            ring.lineWidth = 1
            ring.stroke()
            image.unlockFocus()
            return image
        }
        var overlay: CachedTileOverlay?

        func mapView(_ mapView: MKMapView, rendererFor overlay: MKOverlay) -> MKOverlayRenderer {
            MissionMap.rendererCalls += 1
            MissionMap.rendererKinds.insert(String(describing: type(of: overlay)))
            if let tiles = overlay as? CachedTileOverlay {
                return MKTileOverlayRenderer(tileOverlay: tiles)
            }
            if let orbit = overlay as? OrbitCircle {
                let renderer = MKCircleRenderer(circle: orbit)
                renderer.strokeColor = .systemOrange
                renderer.fillColor = NSColor.systemOrange.withAlphaComponent(0.12)
                renderer.lineWidth = 2
                return renderer
            }
            if let transect = overlay as? TransectPolyline {
                let renderer = MKPolylineRenderer(polyline: transect)
                renderer.strokeColor = NSColor.controlAccentColor.withAlphaComponent(0.55)
                renderer.lineWidth = 1.5
                return renderer
            }
            if let corridor = overlay as? CorridorPolyline {
                let renderer = MKPolylineRenderer(polyline: corridor)
                renderer.strokeColor = .controlAccentColor
                renderer.lineWidth = 4
                renderer.lineDashPattern = [8, 6]
                return renderer
            }
            if let survey = overlay as? SurveyPolygon {
                let renderer = MKPolygonRenderer(polygon: survey)
                renderer.strokeColor = .controlAccentColor
                renderer.fillColor = NSColor.controlAccentColor.withAlphaComponent(0.12)
                renderer.lineWidth = 2
                return renderer
            }
            if let polygon = overlay as? FencePolygon {
                return Coordinator.fenceRenderer(MKPolygonRenderer(polygon: polygon),
                                                 inclusion: polygon.inclusion)
            }
            if let enforced = overlay as? FirmwareFenceCircle {
                let renderer = MKCircleRenderer(circle: enforced)
                renderer.strokeColor = .systemPurple
                renderer.lineWidth = 2
                renderer.lineDashPattern = [6, 4]
                renderer.fillColor = NSColor.systemPurple.withAlphaComponent(0.05)
                return renderer
            }
            if let circle = overlay as? FenceCircle {
                return Coordinator.fenceRenderer(MKCircleRenderer(circle: circle),
                                                 inclusion: circle.inclusion)
            }
            if let flown = overlay as? TrackPolyline {
                let renderer = MKPolylineRenderer(polyline: flown)
                renderer.strokeColor = NSColor(Overlay.vehicle).withAlphaComponent(0.85)
                renderer.lineWidth = 2
                return renderer
            }
            guard let line = overlay as? MKPolyline else { return MKOverlayRenderer(overlay: overlay) }
            let renderer = MKPolylineRenderer(polyline: line)
            renderer.strokeColor = .controlAccentColor
            renderer.lineWidth = 3
            return renderer
        }

        static func fenceRenderer(_ renderer: MKOverlayPathRenderer, inclusion: Bool) -> MKOverlayRenderer {
            let colour = inclusion ? NSColor.systemOrange : NSColor.systemRed
            renderer.strokeColor = colour
            renderer.fillColor = colour.withAlphaComponent(0.12)
            renderer.lineWidth = 2
            return renderer
        }

        func mapView(_ mapView: MKMapView, viewFor annotation: MKAnnotation) -> MKAnnotationView? {
            if let rally = annotation as? RallyAnnotation {
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: "rally") as? MKMarkerAnnotationView
                    ?? MKMarkerAnnotationView(annotation: rally, reuseIdentifier: "rally")
                view.annotation = rally
                view.canShowCallout = true
                view.glyphText = "R"
                view.markerTintColor = .systemGreen
                return view
            }

            if let vertex = annotation as? VertexAnnotation {
                let id = vertex.midpoint ? "midpoint" : "vertex"
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: id)
                    ?? MKAnnotationView(annotation: vertex, reuseIdentifier: id)
                view.annotation = vertex
                view.image = vertex.midpoint ? Coordinator.midpointDot : Coordinator.vertexDot
                view.isDraggable = !vertex.midpoint
                view.canShowCallout = true
                view.rightCalloutAccessoryView = vertex.removable
                    ? Coordinator.removeButton(hint: vertex.hint) : nil
                return view
            }

            if let clicked = annotation as? ClickAnnotation {
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: "click")
                    ?? MKAnnotationView(annotation: clicked, reuseIdentifier: "click")
                view.annotation = clicked
                view.image = Coordinator.clickRing
                view.canShowCallout = true
                return view
            }

            if let looking = annotation as? RoiAnnotation {
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: "roi") as? MKMarkerAnnotationView
                    ?? MKMarkerAnnotationView(annotation: looking, reuseIdentifier: "roi")
                view.annotation = looking
                view.canShowCallout = true
                view.glyphImage = NSImage(systemSymbolName: "viewfinder",
                                          accessibilityDescription: nil)
                view.markerTintColor = .systemOrange
                return view
            }

            if let going = annotation as? GotoAnnotation {
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: "goto") as? MKMarkerAnnotationView
                    ?? MKMarkerAnnotationView(annotation: going, reuseIdentifier: "goto")
                view.annotation = going
                view.canShowCallout = true
                view.glyphImage = NSImage(systemSymbolName: "arrow.right.to.line",
                                          accessibilityDescription: nil)
                view.markerTintColor = .systemBlue
                return view
            }

            if let vehicle = annotation as? VehicleAnnotation {
                let view = mapView.dequeueReusableAnnotationView(withIdentifier: "vehicle")
                    ?? MKAnnotationView(annotation: vehicle, reuseIdentifier: "vehicle")
                view.annotation = vehicle
                view.image = vehicle.hasHeading ? Coordinator.vehicleArrow : Coordinator.vehicleDot
                view.canShowCallout = true
                view.wantsLayer = true
                view.layer?.transform = CATransform3DMakeRotation(vehicle.rotation, 0, 0, 1)
                return view
            }

            guard let item = annotation as? MissionAnnotation else { return nil }
            let view = mapView.dequeueReusableAnnotationView(withIdentifier: "item") as? MKMarkerAnnotationView
                ?? MKMarkerAnnotationView(annotation: item, reuseIdentifier: "item")
            view.annotation = item
            view.canShowCallout = true
            view.isDraggable = item.canMove
            view.glyphText = String(item.sequence)
            view.markerTintColor = item.isLaunch ? .systemGreen : .controlAccentColor
            view.displayPriority = item.isSelected ? .required : .defaultHigh
            view.zPriority = item.isSelected ? .max : .defaultUnselected
            return view
        }

        private static let vehicleArrow: NSImage = {
            let size = NSSize(width: 20, height: 20)
            let image = NSImage(size: size)
            image.lockFocus()
            let arrow = NSBezierPath()
            arrow.move(to: NSPoint(x: 10, y: 19))
            arrow.line(to: NSPoint(x: 3, y: 1))
            arrow.line(to: NSPoint(x: 10, y: 6))
            arrow.line(to: NSPoint(x: 17, y: 1))
            arrow.close()
            NSColor.systemRed.setFill()
            arrow.fill()
            NSColor.white.setStroke()
            arrow.lineWidth = 1.5
            arrow.stroke()
            image.unlockFocus()
            return image
        }()

        private static let vehicleDot: NSImage = {
            let size = NSSize(width: 14, height: 14)
            let image = NSImage(size: size)
            image.lockFocus()
            NSColor.systemRed.setFill()
            NSBezierPath(ovalIn: NSRect(origin: .zero, size: size)).fill()
            NSColor.white.setStroke()
            let ring = NSBezierPath(ovalIn: NSRect(x: 1, y: 1, width: size.width - 2, height: size.height - 2))
            ring.lineWidth = 2
            ring.stroke()
            image.unlockFocus()
            return image
        }()
    }
}
