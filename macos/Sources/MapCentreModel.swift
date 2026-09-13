import Foundation

enum LocationAccess: Equatable {
    case unknown
    case waiting
    case notAsked
    case refused
}

struct GcsFix: Equatable {
    let point: GeoPoint?
    let usable: Bool
    let fix: String
    let source: String
    let imprecise: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        usable = (json["usable"] as? NSNumber)?.boolValue ?? false
        fix = (json["fix"] as? String) ?? ""
        source = (json["source"] as? String) ?? ""
        imprecise = MapCentre.beyondDeclaredAccuracy(json)
        point = MapCentre.centreable(json)
    }
}

struct MapCentreState: Equatable {
    var missionPoints: [GeoPoint] = []
    var otherPoints: [GeoPoint] = []
    var launch: GeoPoint?
    var vehicle: GeoPoint?
    var gcs: GeoPoint?
    var gcsFix = ""
    var gcsImprecise = false
    var access = LocationAccess.unknown

    var allPoints: [GeoPoint] { missionPoints + otherPoints }
}

enum MapCentre: String, CaseIterable, Identifiable {
    case mission
    case allItems
    case launch
    case vehicle
    case myLocation
    case coordinates

    var id: String { rawValue }

    var title: String {
        switch self {
        case .mission: return "Mission"
        case .allItems: return "Everything"
        case .launch: return "Launch"
        case .vehicle: return "Vehicle"
        case .myLocation: return "My Location"
        case .coordinates: return "Coordinates\u{2026}"
        }
    }

    func enabled(in state: MapCentreState) -> Bool {
        switch self {
        case .mission: return !state.missionPoints.isEmpty
        case .allItems: return !state.allPoints.isEmpty
        case .launch: return state.launch != nil
        case .vehicle: return state.vehicle != nil
        case .myLocation: return state.gcs != nil
        case .coordinates: return true
        }
    }

    // A greyed row with no reason leaves the operator guessing whether it is broken or
    // just waiting, and My Location is the one whose cause they can actually fix.
    func note(in state: MapCentreState) -> String {
        guard !enabled(in: state) else { return "" }
        switch self {
        case .mission: return "This plan has no waypoints"
        case .allItems: return "Nothing has been placed yet"
        case .launch: return "No launch position yet"
        case .vehicle: return "No vehicle position yet"
        case .coordinates: return ""
        case .myLocation:
            switch state.access {
            case .notAsked: return "macOS has not been asked for location access yet"
            case .refused: return "macOS is not allowing location access"
            case .waiting: return MapCentre.without(fix: state.gcsFix,
                                                    imprecise: state.gcsImprecise)
            case .unknown: return "No position for this computer"
            }
        }
    }

    static let noSource = "noSource"
    static let stalePrefix = "stale"

    static let waitingToken = "waiting"

    // Accuracy is asked FIRST because it is the only thing that now keeps a position off this
    // map. Staleness used to, and the sentence order still reflected that: a reading that was
    // both old and imprecise would be reported as old, and the operator would wait for a refresh
    // that changes nothing. Age no longer decides anything here, so it can no longer explain
    // anything either.
    static func without(fix: String, imprecise: Bool) -> String {
        if imprecise {
            return "The position is not precise enough to use"
        }
        if fix == MapCentre.noSource {
            return "Nothing is reporting a position for this computer"
        }
        // Every remaining way to disable this row is a position we do not have yet: never fixed,
        // or fixed and expired with no last known reading behind it. The second is unreachable as
        // far as the core's own module goes -- a stale token means a reading existed and
        // lastKnown* survives it -- but the head cannot prove that, so it answers rather than
        // trapping.
        return "Waiting for a position fix"
    }

    // The core's `usable` is fixed AND fresh within five seconds AND inside its declared accuracy
    // floor -- thresholds for a position you would NAVIGATE on. Centring a map is not that task,
    // and gcsposition.rs keeps lastKnownLatitude/lastKnownLongitude readable precisely so a
    // consumer with a different question can ask it. This is `usable` minus the freshness term,
    // and nothing else: the accuracy limit is still the core's own declared number, never one
    // invented here for a purpose the core did not size it for.
    static func centreable(_ json: [String: Any]) -> GeoPoint? {
        guard (json["hasFix"] as? NSNumber)?.boolValue == true,
              !MapCentre.beyondDeclaredAccuracy(json),
              let latitude = (json["lastKnownLatitude"] as? NSNumber)?.doubleValue,
              let longitude = (json["lastKnownLongitude"] as? NSNumber)?.doubleValue else {
            return nil
        }
        return MapCentre.usable(["latitude": latitude as NSNumber,
                                 "longitude": longitude as NSNumber])
    }

    // An unreported accuracy is not a good one. The core refuses a fix whose accuracy it cannot
    // read, and a map centred on a position of unknown precision is the same wrong answer drawn
    // more confidently.
    static func beyondDeclaredAccuracy(_ json: [String: Any]) -> Bool {
        guard (json["hasFix"] as? NSNumber)?.boolValue == true else { return false }
        guard let floor = (json["minimumHorizontalAccuracy"] as? NSNumber)?.doubleValue,
              let reported = (json["horizontalAccuracy"] as? NSNumber)?.doubleValue else {
            return true
        }
        return reported > floor
    }

    func frame(in state: MapCentreState) -> MapFrame? {
        let points: [GeoPoint]
        switch self {
        case .mission: points = state.missionPoints
        case .allItems: points = state.allPoints
        case .launch: points = state.launch.map { [$0] } ?? []
        case .vehicle: points = state.vehicle.map { [$0] } ?? []
        case .myLocation: points = state.gcs.map { [$0] } ?? []
        case .coordinates: return nil
        }
        guard !points.isEmpty else { return nil }
        return MapFrame(latitudes: points.map(\.latitude), longitudes: points.map(\.longitude))
    }

    static func usable(_ coordinate: [String: Any]?) -> GeoPoint? {
        guard let coordinate,
              (coordinate["valid"] as? NSNumber)?.boolValue ?? true,
              let point = GeoPoint(json: coordinate),
              point.latitude != 0 || point.longitude != 0 else { return nil }
        return point
    }

    static func frame(latitude: Double, longitude: Double) -> MapFrame? {
        guard let point = GeoPoint(json: ["latitude": latitude as NSNumber,
                                          "longitude": longitude as NSNumber]),
              abs(point.latitude) <= 90, abs(point.longitude) <= 180 else { return nil }
        return MapFrame(latitudes: [point.latitude], longitudes: [point.longitude])
    }
}
