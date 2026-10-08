import MapLibre
import UIKit

private let HIT_RADIUS_PX: Float = 44
private let DRAG_WRITE_INTERVAL_MS: Double = 120
let TAP_SLOP_PX: Float = 20

enum MapHit: Equatable, Hashable {
    case Waypoint(index: Int)
    case FenceVertex(polygon: Int, vertex: Int)
    case SurveyVertex(item: Int, vertex: Int)
    case Rally(index: Int)
    case BreachReturn
    case Circle(index: Int)
    case CircleCentre(index: Int)
    case CircleRadius(index: Int)
    case LandingPlace(index: Int, place: Int)
    case Midpoint(path: String, invokable: String, segment: Int)
    case ShapeCentre(fence: Bool, owner: Int)
    case ShapeRadius(fence: Bool, owner: Int)
    case LoiterRadius(index: Int)
    case LoiterRotation(index: Int)
}

func nearestIndex(_ x: Float, _ y: Float, _ points: [(Float, Float)?]) -> Int? {
    points.indices.min { distanceSquared(x, y, points[$0]) < distanceSquared(x, y, points[$1]) }
}

private func distanceSquared(_ x: Float, _ y: Float, _ point: (Float, Float)?) -> Float {
    guard let (px, py) = point else { return .greatestFiniteMagnitude }
    return (px - x) * (px - x) + (py - y) * (py - y)
}

private func nearest(_ map: MLNMapView, _ features: [MLNFeature], _ x: Float, _ y: Float) -> MLNFeature? {
    nearestIndex(x, y, features.map { feature in
        (feature as? MLNPointFeature).map { point in
            let screen = map.convert(point.coordinate, toPointTo: map)
            return (Float(screen.x), Float(screen.y))
        }
    }).map { features[$0] }
}

private func stringProperty(_ feature: MLNFeature, _ key: String) -> String? {
    feature.attribute(forKey: key) as? String
}

private func intProperty(_ feature: MLNFeature, _ key: String) -> Int? {
    (feature.attribute(forKey: key) as? NSNumber)?.intValue
}

func handleHit(_ kind: String?, _ owner: Int, _ vertex: Int) -> MapHit? {
    switch kind {
    case HANDLE_KIND_FENCE: .FenceVertex(polygon: owner, vertex: vertex)
    case HANDLE_KIND_SURVEY: .SurveyVertex(item: owner, vertex: vertex)
    case HANDLE_KIND_CIRCLE: .CircleCentre(index: owner)
    case HANDLE_KIND_FENCE_CIRCLE_RADIUS: .CircleRadius(index: owner)
    case HANDLE_KIND_LANDING: .LandingPlace(index: owner, place: vertex)
    case HANDLE_KIND_FENCE_CENTRE: .ShapeCentre(fence: true, owner: owner)
    case HANDLE_KIND_SURVEY_CENTRE: .ShapeCentre(fence: false, owner: owner)
    case HANDLE_KIND_CIRCLE_RADIUS: .ShapeRadius(fence: vertex == 0, owner: owner)
    case HANDLE_KIND_LOITER_RADIUS: .LoiterRadius(index: owner)
    case HANDLE_KIND_LOITER_ROTATION: .LoiterRotation(index: owner)
    default: nil
    }
}

private func hitRadius(_ map: MLNMapView) -> CGFloat {
    CGFloat(HIT_RADIUS_PX) / max(map.contentScaleFactor, 1)
}

func hitTest(_ map: MLNMapView, _ x: Float, _ y: Float) -> MapHit? {
    let radius = hitRadius(map)
    let box = CGRect(x: CGFloat(x) - radius, y: CGFloat(y) - radius, width: radius * 2, height: radius * 2)
    func features(_ layers: String...) -> [MLNFeature] {
        map.visibleFeatures(in: box, styleLayerIdentifiers: Set(layers))
    }

    if let feature = nearest(map, features(MIDPOINT_LAYER), x, y),
       let path = stringProperty(feature, SHAPE_PATH_PROPERTY), !path.isBlank,
       let invokable = stringProperty(feature, SPLIT_INVOKABLE_PROPERTY), !invokable.isBlank,
       let segment = intProperty(feature, VERTEX_INDEX_PROPERTY) {
        return .Midpoint(path: path, invokable: invokable, segment: segment)
    }

    if let feature = nearest(map, features(FENCE_HANDLE_LAYER), x, y),
       let owner = intProperty(feature, POLYGON_INDEX_PROPERTY),
       let vertex = intProperty(feature, VERTEX_INDEX_PROPERTY),
       let hit = handleHit(stringProperty(feature, HANDLE_KIND_PROPERTY), owner, vertex) {
        return hit
    }

    if !features(BREACH_LAYER).isEmpty {
        return .BreachReturn
    }

    if let feature = nearest(map, features(RALLY_LAYER), x, y), let index = intProperty(feature, RALLY_INDEX_PROPERTY) {
        return .Rally(index: index)
    }

    if let circle = features(FENCE_FILL_LAYER).lazy.compactMap({ intProperty($0, CIRCLE_INDEX_PROPERTY) }).first {
        return .Circle(index: circle)
    }

    return nearest(map, features(MISSION_DOT_LAYER, MISSION_LAYER), x, y)
        .flatMap { intProperty($0, WAYPOINT_ID_PROPERTY) }
        .map { .Waypoint(index: $0) }
}

func withinHit(_ dx: Float, _ dy: Float) -> Bool {
    abs(dx) <= HIT_RADIUS_PX && abs(dy) <= HIT_RADIUS_PX
}

func withinTap(_ dx: Float, _ dy: Float) -> Bool {
    abs(dx) <= TAP_SLOP_PX && abs(dy) <= TAP_SLOP_PX
}

func attachMissionEditing(
    _ mapView: MLNMapView,
    _ map: MLNMapView,
    _ style: MLNStyle,
    onAdd: @escaping (Double, Double) -> Void,
    onMove: @escaping (MapHit, Double, Double) -> Void,
    onSelected: @escaping (MapHit?) -> Void = { _ in },
    onMoved: @escaping (MapHit, Double, Double) -> Void = { _, _, _ in },
    onDragging: @escaping (MapHit?) -> Void = { _ in },
    canDrag: @escaping (MapHit) -> Bool = { _ in true }
) {
    mapView.gestureRecognizers?.filter { $0 is MissionEditingGesture }.forEach(mapView.removeGestureRecognizer)
    let editing = MissionEditingGesture(map: map, onAdd: onAdd, onMove: onMove, onSelected: onSelected, onMoved: onMoved, onDragging: onDragging, canDrag: canDrag)
    let longPress = UILongPressGestureRecognizer(target: editing, action: #selector(MissionEditingGesture.longPressed(_:)))
    longPress.delegate = editing
    editing.longPress = longPress
    mapView.addGestureRecognizer(editing)
    mapView.addGestureRecognizer(longPress)
}

private final class MissionEditingGesture: UIGestureRecognizer, UIGestureRecognizerDelegate {
    private weak var map: MLNMapView?
    private let onAdd: (Double, Double) -> Void
    private let onMove: (MapHit, Double, Double) -> Void
    private let onSelected: (MapHit?) -> Void
    private let onMoved: (MapHit, Double, Double) -> Void
    private let onDragging: (MapHit?) -> Void
    private let canDrag: (MapHit) -> Bool
    weak var longPress: UILongPressGestureRecognizer?

    private var dragging: MapHit?
    private var tapped: MapHit?
    private var down = CGPoint.zero
    private var moved = false
    private var lastWriteAt: CFTimeInterval = 0
    private var addedInGesture = false

    init(
        map: MLNMapView,
        onAdd: @escaping (Double, Double) -> Void,
        onMove: @escaping (MapHit, Double, Double) -> Void,
        onSelected: @escaping (MapHit?) -> Void,
        onMoved: @escaping (MapHit, Double, Double) -> Void,
        onDragging: @escaping (MapHit?) -> Void,
        canDrag: @escaping (MapHit) -> Bool
    ) {
        self.map = map
        self.onAdd = onAdd
        self.onMove = onMove
        self.onSelected = onSelected
        self.onMoved = onMoved
        self.onDragging = onDragging
        self.canDrag = canDrag
        super.init(target: nil, action: nil)
        cancelsTouchesInView = false
        delaysTouchesBegan = false
        delaysTouchesEnded = false
        delegate = self
    }

    func gestureRecognizer(_ gestureRecognizer: UIGestureRecognizer, shouldRecognizeSimultaneouslyWith other: UIGestureRecognizer) -> Bool {
        true
    }

    @objc func longPressed(_ press: UILongPressGestureRecognizer) {
        guard press.state == .began, dragging == nil, let map else { return }
        addedInGesture = true
        if let tapped {
            onSelected(tapped)
        } else {
            let at = map.convert(press.location(in: map), toCoordinateFrom: map)
            onAdd(at.latitude, at.longitude)
        }
    }

    private func pixels(_ point: CGPoint) -> (Float, Float) {
        let scale = Float(max(map?.contentScaleFactor ?? 1, 1))
        return (Float(point.x - down.x) * scale, Float(point.y - down.y) * scale)
    }

    private func setMapGestures(_ enabled: Bool) {
        map?.isScrollEnabled = enabled
        map?.isZoomEnabled = enabled
        map?.isRotateEnabled = enabled
        map?.isPitchEnabled = enabled
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent) {
        guard let map, touches.count == 1, (event.allTouches?.count ?? 1) == 1, let touch = touches.first else {
            if dragging == nil { state = .failed }
            return
        }
        let at = touch.location(in: map)
        let hit = hitTest(map, Float(at.x), Float(at.y))
        down = at
        moved = false
        addedInGesture = false
        tapped = hit.flatMap { canDrag($0) ? nil : $0 }
        guard let hit, tapped == nil else {
            setMapGestures(true)
            return
        }
        dragging = hit
        setMapGestures(false)
        state = .began
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent) {
        guard let map, let touch = touches.first else { return }
        let at = touch.location(in: map)
        let (dx, dy) = pixels(at)
        guard let hit = dragging else {
            if !withinTap(dx, dy) { state = .failed }
            return
        }
        state = .changed
        if case .Midpoint = hit { return }
        if !moved && !withinTap(dx, dy) {
            moved = true
            onDragging(hit)
        }
        let now = CACurrentMediaTime()
        if moved && (now - lastWriteAt) * 1000 >= DRAG_WRITE_INTERVAL_MS {
            lastWriteAt = now
            let target = map.convert(at, toCoordinateFrom: map)
            onMove(hit, target.latitude, target.longitude)
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent) {
        finish(touches, released: true)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent) {
        finish(touches, released: false)
    }

    private func finish(_ touches: Set<UITouch>, released: Bool) {
        guard let map, let touch = touches.first else { return reset() }
        let at = touch.location(in: map)
        let (dx, dy) = pixels(at)
        let hit = dragging
        dragging = nil
        setMapGestures(true)
        if moved { onDragging(nil) }
        let tap = tapped
        tapped = nil
        guard let hit else {
            if !addedInGesture && released && withinTap(dx, dy) { onSelected(tap) }
            state = .failed
            return
        }
        if moved {
            let target = map.convert(at, toCoordinateFrom: map)
            onMoved(hit, target.latitude, target.longitude)
        } else if !isMidpoint(hit) || withinTap(dx, dy) {
            onSelected(hit)
        }
        state = released ? .ended : .cancelled
    }

    private func isMidpoint(_ hit: MapHit) -> Bool {
        if case .Midpoint = hit { return true }
        return false
    }

    override func reset() {
        super.reset()
        if dragging != nil { setMapGestures(true) }
        dragging = nil
    }
}
