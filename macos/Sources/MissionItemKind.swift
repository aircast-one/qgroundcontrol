import Foundation

struct MissionItemKind: Identifiable, Equatable {
    let id: String
    let title: String
    let invokable: String
    let complexName: String?
    let geometry: String?
    let geometryProperty: String?
    let shapeNoun: String
    let placementHint: String
    let simple: Bool
    let enabled: Bool
    let disabledReason: String?

    var symbol: String { MissionItemKind.symbol(forId: id) }

    static func symbol(forId id: String) -> String {
        switch id {
        case "waypoint": return "mappin"
        case "takeoff": return "arrow.up.right"
        case "land": return "arrow.down.right"
        case "roi": return "eye"
        case "survey": return "square.grid.3x3"
        case "corridor": return "road.lanes"
        case "structure": return "building.2"
        default: return MissionKinds.unknownSymbol
        }
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = json["id"] as? String,
              let title = json["title"] as? String else { return nil }
        self.id = id
        self.title = title
        invokable = (json["invokable"] as? String) ?? ""
        complexName = json["complexName"] as? String
        geometry = json["geometry"] as? String
        geometryProperty = json["geometryProperty"] as? String
        shapeNoun = (json["shapeNoun"] as? String) ?? "shape"
        placementHint = (json["placementHint"] as? String) ?? ""
        simple = (json["simple"] as? NSNumber)?.boolValue ?? false
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? false
        disabledReason = json["disabledReason"] as? String
    }
}

struct MissionKinds: Equatable {
    let all: [MissionItemKind]

    static let empty = MissionKinds(all: [])
    static let unknownSymbol = "square.on.square.dashed"

    init(all: [MissionItemKind]) { self.all = all }

    init(_ json: [String: Any]) {
        all = ((json["kinds"] as? [Any]) ?? []).compactMap(MissionItemKind.init)
    }

    func byId(_ id: String) -> MissionItemKind? { all.first { $0.id == id } }

    func byComplexName(_ name: String) -> MissionItemKind? {
        all.first { $0.complexName == name }
    }

    var simple: [MissionItemKind] { all.filter(\.simple) }

    var shapeImportable: [MissionItemKind] { all.filter { !$0.simple } }

    // QGC has complex items the catalogue does not name — a Landing Pattern is one —
    // so a plan can carry a pattern the core cannot describe, and it still has to draw.
    func title(forPattern name: String) -> String {
        byComplexName(name)?.title ?? name
    }

    func symbol(forPattern name: String) -> String {
        byComplexName(name).map(\.symbol) ?? MissionKinds.unknownSymbol
    }

    func placementHint(forPattern name: String) -> String {
        byComplexName(name)?.placementHint
            ?? "Click the map to place a \(name.lowercased())."
    }

    func areaProperty(forCommand command: String) -> String? {
        byComplexName(command).flatMap { $0.geometry == "area" ? $0.geometryProperty : nil }
    }

    func lineProperty(forCommand command: String) -> String? {
        byComplexName(command).flatMap { $0.geometry == "line" ? $0.geometryProperty : nil }
    }

    // A pattern the catalogue does not name is not one the core refused, so it stays offered.
    func offers(pattern name: String) -> Bool {
        byComplexName(name)?.enabled ?? true
    }

    func refusal(pattern name: String) -> String? {
        byComplexName(name)?.disabledReason
    }

    static let refusedWithoutReason = "That item cannot go here."
}

// What mission.insert answered. The core knows whether it holds a kind and whether the plan will
// take it now; a head that decides from its own polled catalogue is guessing with a stale copy,
// and QGC has item types the catalogue does not list at all.
enum InsertOutcome: Equatable {
    case inserted
    case insertDirectly(String)
    case refused(String)

    init(_ answer: [String: Any]) {
        if (answer["ok"] as? NSNumber)?.boolValue == true {
            self = .inserted
        } else if let name = answer["unknown"] as? String {
            // lookup() fails before the core touches the controller, so nothing was inserted and
            // nothing needs undoing -- the item simply goes in the way it always did.
            self = .insertDirectly(name)
        } else {
            self = .refused((answer["reason"] as? String) ?? MissionKinds.refusedWithoutReason)
        }
    }
}

struct MissionSeed: Equatable {
    let property: String
    let points: [GeoPoint]

    init?(_ json: [String: Any]) {
        guard let property = json["property"] as? String,
              let listed = json["points"] as? [Any], !listed.isEmpty else { return nil }
        self.property = property
        points = listed.compactMap { entry in
            guard let entry = entry as? [String: Any],
                  let latitude = (entry["latitude"] as? NSNumber)?.doubleValue,
                  let longitude = (entry["longitude"] as? NSNumber)?.doubleValue else { return nil }
            return GeoPoint(latitude: latitude, longitude: longitude)
        }
    }
}
