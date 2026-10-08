import SwiftUI

@Observable
final class MapBridgeReadiness {
    fileprivate(set) var ready = false
}

enum MapBridge {
    @MainActor static let readiness = MapBridgeReadiness()

    @MainActor static var bridgeReady: Bool { readiness.ready }

    static func start() {
        markReachable()
    }

    @MainActor static func watch(_ path: String) {
        _ = QgcWatch.retain(path)
    }

    @MainActor static func unwatch(_ path: String) {
        QgcWatch.release(path)
    }

    @MainActor static var watchedPaths: Set<String> { QgcWatch.watched }

    static func seed(_ path: String) {
        offMain {
            guard let json = read(path) else { return }
            onMain { QgcWatch.seed(path, json) }
        }
    }

    static func read(_ path: String) -> JSON? {
        let json = Qgc.get(path)
        return json.object != nil ? json : nil
    }

    static func markReachable() {
        onMain { readiness.ready = true }
    }
}

private final class SeededPath {
    var path: String?
}

@propertyWrapper
struct MapPath: DynamicProperty {
    @QgcPath private var json: JSON?
    @State private var seeded = SeededPath()
    private let path: String?

    init(_ path: String?) {
        _json = QgcPath(path)
        self.path = path
    }

    var wrappedValue: JSON? { json }

    nonisolated func update() {
        MainActor.assumeIsolated {
            guard let path, seeded.path != path else { return }
            seeded.path = path
            MapBridge.seed(path)
        }
    }
}

@propertyWrapper
struct MapDouble: DynamicProperty {
    @MapPath private var json: JSON?

    init(_ path: String?) { _json = MapPath(path) }

    var wrappedValue: Double {
        switch json?["value"] {
        case .number(let value): value
        case .string(let text): Double(text) ?? .nan
        default: .nan
        }
    }
}

@propertyWrapper
struct MapString: DynamicProperty {
    @MapPath private var json: JSON?

    init(_ path: String?) { _json = MapPath(path) }

    var wrappedValue: String { json?["value"].stringOrNil ?? "" }
}

@propertyWrapper
struct MapInt: DynamicProperty {
    @MapPath private var json: JSON?
    private let fallback: Int

    init(_ path: String?, fallback: Int = -1) {
        _json = MapPath(path)
        self.fallback = fallback
    }

    var wrappedValue: Int {
        switch json?["value"] {
        case .number(let value): Int(exactly: value.rounded(.towardZero)) ?? fallback
        case .string(let text): Int(text) ?? fallback
        default: fallback
        }
    }
}

@propertyWrapper
struct MapCount: DynamicProperty {
    @MapPath private var json: JSON?

    init(_ path: String?) { _json = MapPath(path) }

    var wrappedValue: Int { json?["elements"].arrayOrNil?.count ?? 0 }
}

func coordinateOf(_ json: JSON?) -> TrackPoint? {
    guard let json, json["valid"].bool else { return nil }
    let latitude = json["latitude"].double ?? .nan
    let longitude = json["longitude"].double ?? .nan
    return isPlottable(latitude, longitude) ? TrackPoint(latitude: latitude, longitude: longitude) : nil
}

let FLY_STATE_VIEW = "view.flyState"

@propertyWrapper
struct MapCoordinate: DynamicProperty {
    @MapPath private var json: JSON?

    init(_ path: String?) { _json = MapPath(path) }

    var wrappedValue: TrackPoint? { coordinateOf(json) }
}

@propertyWrapper
struct MapViewFlag: DynamicProperty {
    @MapPath private var json: JSON?
    private let field: String

    init(_ path: String?, _ field: String) {
        _json = MapPath(path)
        self.field = field
    }

    var wrappedValue: Bool { json?[field].bool == true }
}

@propertyWrapper
struct MapBool: DynamicProperty {
    @MapPath private var json: JSON?

    init(_ path: String?) { _json = MapPath(path) }

    var wrappedValue: Bool {
        switch json?["value"] {
        case .bool(true), .string("true"): true
        default: false
        }
    }
}
