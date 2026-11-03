import MapLibre
#if os(iOS)
import UIKit
#elseif os(macOS)
import AppKit
#endif

let GOTO_MAP_CLICK_VIEW = "view.mapClick"
private let GOTO_SOURCE = "aircast-goto"
private let GOTO_RING_SOURCE = "aircast-goto-ring"
private let GOTO_LAYER = "aircast-goto-layer"
private let GOTO_LABEL_LAYER = "aircast-goto-label"
private let GOTO_RING_LAYER = "aircast-goto-ring-layer"
private let GOTO_RADIUS_SOURCE = "aircast-goto-radius"
private let GOTO_RADIUS_LAYER = "aircast-goto-radius-layer"
private let RADIUS_TEXT = "text"
private let GOTO_ARROW_SOURCE = "aircast-goto-arrows"
private let GOTO_ARROW_LAYER = "aircast-goto-arrow-layer"
private let ARROW_BEARING = "bearing"
private let GOTO_HANDLE_SOURCE = "aircast-goto-handle"
private let GOTO_HANDLE_LAYER = "aircast-goto-handle-layer"
private let GOTO_COLOUR = "#2E7D32"
private let GOTO_PATH_SOURCE = "aircast-goto-path"
private let GOTO_PATH_LAYER = "aircast-goto-path-layer"
private let PATH_DASH = 2.0
private let PATH_GAP = 1.5

func previewGoto(_ confirmed: GotoLocation?, _ preview: TrackPoint?) -> GotoLocation? {
    preview.map { GotoLocation(at: $0, loiterRadiusMetres: nil) } ?? confirmed
}

func gotoPath(_ from: TrackPoint?, _ preview: TrackPoint?) -> [TrackPoint] {
    let path = [from.flatMap { isPlottable($0.latitude, $0.longitude) ? $0 : nil }, preview].compactMap { $0 }
    return path.count == 2 ? path : []
}

struct GotoLocation: Equatable {
    var at: TrackPoint
    var loiterRadiusMetres: Double?
    var loiterRadiusText: String = ""
    var loiterClockwise: Bool = true
}

func gotoLocation(_ view: JSON?) -> GotoLocation? {
    guard let json = view?["gotoLocation"], json.object != nil else { return nil }
    let radius = json["loiterRadiusMetres"].double(.nan)
    let location = GotoLocation(
        at: TrackPoint(latitude: json["latitude"].double(.nan), longitude: json["longitude"].double(.nan)),
        loiterRadiusMetres: !radius.isNaN && radius > 0 ? radius : nil,
        loiterRadiusText: json["loiterRadiusText"].string,
        loiterClockwise: json["loiterClockwise"].bool(true)
    )
    return isPlottable(location.at.latitude, location.at.longitude) ? location : nil
}

func gotoRing(_ location: GotoLocation?) -> [TrackPoint] {
    let ring = location.flatMap { at in at.loiterRadiusMetres.map { circleRing(at.at, $0) } } ?? []
    return ring.first.map { ring + [$0] } ?? []
}

func gotoArrows(_ location: GotoLocation?) -> [(TrackPoint, Double)] {
    orbitArrows(location.flatMap { at in at.loiterRadiusMetres.map { OrbitCircle(centre: at.at, radiusMetres: $0, clockwise: at.loiterClockwise) } })
}

struct LoiterEdit: Equatable {
    var radiusMetres: Double
    var clockwise: Bool
    var unit: String
    var metresPerUnit: Double
}

func loiterEditNumber(_ edit: LoiterEdit) -> String {
    String(format: "%.1f", locale: Locale(identifier: "en_US_POSIX"), edit.radiusMetres / edit.metresPerUnit).removingSuffix(".0")
}

func editedGoto(_ location: GotoLocation?, _ edit: LoiterEdit?) -> GotoLocation? {
    guard let changing = edit, let location, location.loiterRadiusMetres != nil else { return location }
    return withChanges(location) {
        $0.loiterRadiusMetres = changing.radiusMetres
        $0.loiterClockwise = changing.clockwise
        $0.loiterRadiusText = [loiterEditNumber(changing), changing.unit].filter { !$0.isBlank }.joined(separator: " ")
    }
}

func gotoRadiusHandle(_ location: GotoLocation?) -> TrackPoint? {
    location.flatMap { at in at.loiterRadiusMetres.map { pointAt(at.at, $0, 90) } }
}

func draggedGotoRadius(_ location: GotoLocation, _ to: TrackPoint) -> Double {
    max(metresBetween(location.at, to), MINIMUM_CIRCLE_RADIUS_METRES)
}

private enum CircleGrab { case None, GotoRadius, GotoFlip, OrbitRadius, OrbitCentre, OrbitFlip }

func attachGotoRadiusDrag(_ map: MLNMapView, _ edits: FlyMapEdits, _ shown: @escaping () -> GotoLocation?) {
    map.addGestureRecognizer(CircleGrabGesture(map: map, edits: edits, shown: shown))
}

#if os(iOS)
private typealias CircleGrabBase = UILongPressGestureRecognizer
#elseif os(macOS)
private typealias CircleGrabBase = NSGestureRecognizer
#endif

private final class CircleGrabGesture: CircleGrabBase, MapGestureDelegate {
    private weak var map: MLNMapView?
    private let edits: FlyMapEdits
    private let shown: () -> GotoLocation?
    private var grab = CircleGrab.None
    private var down = CGPoint.zero

    init(map: MLNMapView, edits: FlyMapEdits, shown: @escaping () -> GotoLocation?) {
        self.map = map
        self.edits = edits
        self.shown = shown
        super.init(target: nil, action: nil)
        #if os(iOS)
        minimumPressDuration = 0
        allowableMovement = .greatestFiniteMagnitude
        delegate = self
        addTarget(self, action: #selector(tracked))
        #elseif os(macOS)
        delaysPrimaryMouseButtonEvents = false
        delegate = self
        #endif
    }

    #if os(macOS)
    required init?(coder: NSCoder) {
        return nil
    }
    #endif

    private var dragging: Bool { [.GotoRadius, .OrbitRadius, .OrbitCentre].contains(grab) }

    private func pixels(_ dx: CGFloat, _ dy: CGFloat) -> (Float, Float) {
        let scale = max(map?.screenScale ?? 1, 1)
        return (Float(dx * scale), Float(dy * scale))
    }

    private func near(_ at: TrackPoint?, _ point: CGPoint) -> Bool {
        guard let at, let map else { return false }
        let screen = map.screenPoint(at.location)
        let (dx, dy) = pixels(screen.x - point.x, screen.y - point.y)
        return withinHit(dx, dy)
    }

    private func nearArrow(_ arrows: [(TrackPoint, Double)], _ point: CGPoint) -> Bool {
        arrows.contains { at, _ in near(at, point) }
    }

    private func grabbed(_ point: CGPoint) -> CircleGrab {
        let orbit = edits.orbit
        let goto = edits.gotoLoiter == nil ? nil : shown()
        return near(orbitRadiusHandle(orbit), point) ? .OrbitRadius
            : near(orbit?.centre, point) ? .OrbitCentre
            : nearArrow(orbitArrows(orbit), point) ? .OrbitFlip
            : near(gotoRadiusHandle(goto), point) ? .GotoRadius
            : nearArrow(gotoArrows(goto), point) ? .GotoFlip
            : .None
    }

    private func dragTo(_ point: CGPoint) {
        guard let map else { return }
        let to = TrackPoint(map.screenCoordinate(point))
        let orbit = edits.orbit
        let edit = edits.gotoLoiter
        let goto = shown()
        switch grab {
        case .OrbitRadius: if let orbit { edits.orbit = withChanges(orbit) { $0.radiusMetres = draggedOrbitRadius(orbit, to) } }
        case .OrbitCentre: if let orbit { edits.orbit = withChanges(orbit) { $0.centre = to } }
        case .GotoRadius: if let edit, let goto { edits.gotoLoiter = withChanges(edit) { $0.radiusMetres = draggedGotoRadius(goto, to) } }
        default: break
        }
    }

    private func flip() {
        let orbit = edits.orbit
        let edit = edits.gotoLoiter
        switch grab {
        case .OrbitFlip: if let orbit { edits.orbit = withChanges(orbit) { $0.clockwise = !orbit.clockwise } }
        case .GotoFlip: if let edit { edits.gotoLoiter = withChanges(edit) { $0.clockwise = !edit.clockwise } }
        default: break
        }
    }

    private func setMapGestures(_ enabled: Bool) {
        map?.isScrollEnabled = enabled
        map?.isZoomEnabled = enabled
        map?.isRotateEnabled = enabled
        map?.isPitchEnabled = enabled
    }

    #if os(iOS)
    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldReceive touch: UITouch) -> Bool {
        guard let map, state == .possible else { return false }
        grab = grabbed(touch.location(in: map))
        return grab != .None
    }

    @objc private func tracked() {
        guard let map else { return }
        track(location(in: map), state)
    }
    #elseif os(macOS)
    // A mouse press on a handle or arrow drives the same began / changed / ended track the iOS long press reports.
    override func mouseDown(with event: NSEvent) {
        guard let map, state == .possible else { return }
        let at = map.screenLocation(event)
        grab = grabbed(at)
        guard grab != .None else {
            state = .failed
            return
        }
        state = .began
        track(at, .began)
    }

    override func mouseDragged(with event: NSEvent) {
        guard let map, state == .began || state == .changed else { return }
        state = .changed
        track(map.screenLocation(event), .changed)
    }

    override func mouseUp(with event: NSEvent) {
        guard let map, state == .began || state == .changed else { return }
        track(map.screenLocation(event), .ended)
        state = .ended
    }
    #endif

    private func track(_ at: CGPoint, _ phase: MapGestureRecognizer.State) {
        switch phase {
        case .began:
            down = at
            if dragging { setMapGestures(false) }
        case .changed:
            if dragging { dragTo(at) }
        case .ended, .cancelled, .failed:
            let (dx, dy) = pixels(at.x - down.x, at.y - down.y)
            if phase == .ended && withinTap(dx, dy) { flip() }
            if dragging { setMapGestures(true) }
            grab = .None
        default:
            break
        }
    }
}

#if os(macOS)
extension CircleGrabGesture: MapExclusiveGesture {}
#endif

enum GotoBridge {
    static func read() -> GotoLocation? { gotoLocation(Qgc.get(GOTO_MAP_CLICK_VIEW)) }
}

func installGotoLayer(_ style: MLNStyle) {
    guard style.source(withIdentifier: GOTO_SOURCE) == nil else { return }
    let target = geoJsonSource(GOTO_SOURCE)
    let ring = geoJsonSource(GOTO_RING_SOURCE)
    let radius = geoJsonSource(GOTO_RADIUS_SOURCE)
    let arrows = geoJsonSource(GOTO_ARROW_SOURCE)
    let handle = geoJsonSource(GOTO_HANDLE_SOURCE)
    let path = geoJsonSource(GOTO_PATH_SOURCE)
    [target, ring, radius, arrows, handle, path].forEach(style.addSource)
    let leg = MLNLineStyleLayer(identifier: GOTO_PATH_LAYER, source: path)
    leg.lineColor = styleConstant(mapColour(GOTO_COLOUR))
    leg.lineWidth = styleConstant(3)
    leg.lineDashPattern = styleConstant([PATH_DASH, PATH_GAP])
    style.addLayer(leg)
    let line = MLNLineStyleLayer(identifier: GOTO_RING_LAYER, source: ring)
    line.lineColor = styleConstant(mapColour(GOTO_COLOUR))
    line.lineWidth = styleConstant(2)
    style.addLayer(line)
    let arrow = MLNSymbolStyleLayer(identifier: GOTO_ARROW_LAYER, source: arrows)
    arrow.text = styleConstant("\u{25B2}")
    arrow.textFontNames = styleConstant(["Noto Sans Regular"])
    arrow.textFontSize = styleConstant(14)
    arrow.textColor = styleConstant(mapColour(GOTO_COLOUR))
    arrow.textRotation = styleGet(ARROW_BEARING)
    arrow.textRotationAlignment = styleConstant("map")
    arrow.textAllowsOverlap = styleConstant(true)
    arrow.textIgnoresPlacement = styleConstant(true)
    style.addLayer(arrow)
    let dot = MLNCircleStyleLayer(identifier: GOTO_LAYER, source: target)
    dot.circleColor = styleConstant(mapColour(GOTO_COLOUR))
    dot.circleRadius = styleConstant(9)
    dot.circleStrokeColor = styleConstant(PlatformColor.white)
    dot.circleStrokeWidth = styleConstant(2)
    style.addLayer(dot)
    let radiusText = MLNSymbolStyleLayer(identifier: GOTO_RADIUS_LAYER, source: radius)
    radiusText.text = styleGet(RADIUS_TEXT)
    radiusText.textFontSize = styleConstant(12)
    radiusText.textColor = styleConstant(PlatformColor.black)
    radiusText.textHaloColor = styleConstant(mapColour("#80FFFFFF"))
    radiusText.textHaloWidth = styleConstant(4)
    radiusText.textAnchor = styleConstant("bottom")
    radiusText.textAllowsOverlap = styleConstant(true)
    radiusText.textIgnoresPlacement = styleConstant(true)
    style.addLayer(radiusText)
    let grip = MLNCircleStyleLayer(identifier: GOTO_HANDLE_LAYER, source: handle)
    grip.circleColor = styleConstant(PlatformColor.white)
    grip.circleRadius = styleConstant(9)
    grip.circleStrokeColor = styleConstant(mapColour(GOTO_COLOUR))
    grip.circleStrokeWidth = styleConstant(3)
    style.addLayer(grip)
    let label = MLNSymbolStyleLayer(identifier: GOTO_LABEL_LAYER, source: target)
    label.text = styleConstant("Go here")
    label.textFontSize = styleConstant(12)
    label.textColor = styleConstant(PlatformColor.white)
    label.textOffset = styleOffset(0, 1.6)
    label.textAnchor = styleConstant("top")
    label.textAllowsOverlap = styleConstant(true)
    label.textIgnoresPlacement = styleConstant(true)
    style.addLayer(label)
}

func renderGotoPath(_ style: MLNStyle, _ path: [TrackPoint]) {
    style.setGeoJson(GOTO_PATH_SOURCE, featureCollection(path.count >= 2 ? [lineFeature(path)] : []))
}

func renderGoto(_ style: MLNStyle, _ location: GotoLocation?, editing: Bool = false) {
    style.setGeoJson(GOTO_HANDLE_SOURCE, featureCollection([gotoRadiusHandle(editing ? location : nil)].compactMap { $0 }.map { pointFeature($0) }))
    style.setGeoJson(GOTO_SOURCE, featureCollection([location].compactMap { $0 }.map { pointFeature($0.at) }))
    style.setGeoJson(
        GOTO_RADIUS_SOURCE,
        featureCollection(
            [location].compactMap { $0 }
                .filter { $0.loiterRadiusMetres != nil && !$0.loiterRadiusText.isBlank }
                .map { shown in pointFeature(pointAt(shown.at, shown.loiterRadiusMetres ?? 0, 0), attributes: [RADIUS_TEXT: shown.loiterRadiusText]) }
        )
    )
    style.setGeoJson(
        GOTO_ARROW_SOURCE,
        featureCollection(gotoArrows(location).map { at, bearing in pointFeature(at, attributes: [ARROW_BEARING: bearing]) })
    )
    style.setGeoJson(GOTO_RING_SOURCE, featureCollection([gotoRing(location)].filter { !$0.isEmpty }.map { lineFeature($0) }))
}

private let CLICK_MARKER_SOURCE = "aircast-click-marker"
private let CLICK_MARKER_SHADOW_LAYER = "aircast-click-marker-shadow"
private let CLICK_MARKER_RING_LAYER = "aircast-click-marker-ring"
private let CLICK_MARKER_DOT_LAYER = "aircast-click-marker-dot"
private let CLICK_MARKER_RADIUS: CGFloat = 13

func installClickMarker(_ style: MLNStyle) {
    guard style.source(withIdentifier: CLICK_MARKER_SOURCE) == nil else { return }
    let source = geoJsonSource(CLICK_MARKER_SOURCE)
    style.addSource(source)
    let shadow = MLNCircleStyleLayer(identifier: CLICK_MARKER_SHADOW_LAYER, source: source)
    shadow.circleRadius = styleConstant(CLICK_MARKER_RADIUS)
    shadow.circleOpacity = styleConstant(0)
    shadow.circleStrokeColor = styleConstant(PlatformColor(white: 0, alpha: 0.6))
    shadow.circleStrokeWidth = styleConstant(4)
    style.addLayer(shadow)
    let ring = MLNCircleStyleLayer(identifier: CLICK_MARKER_RING_LAYER, source: source)
    ring.circleRadius = styleConstant(CLICK_MARKER_RADIUS - 1)
    ring.circleOpacity = styleConstant(0)
    ring.circleStrokeColor = styleConstant(PlatformColor.white)
    ring.circleStrokeWidth = styleConstant(2)
    style.addLayer(ring)
    let dot = MLNCircleStyleLayer(identifier: CLICK_MARKER_DOT_LAYER, source: source)
    dot.circleRadius = styleConstant(2)
    dot.circleColor = styleConstant(PlatformColor.white)
    style.addLayer(dot)
}

func clickMarkerFeatures(_ at: TrackPoint?) -> FeatureCollection {
    featureCollection([at].compactMap { $0 }.filter { isPlottable($0.latitude, $0.longitude) }.map { pointFeature($0) })
}

func renderClickMarker(_ style: MLNStyle, _ at: TrackPoint?) {
    style.setGeoJson(CLICK_MARKER_SOURCE, clickMarkerFeatures(at))
}
