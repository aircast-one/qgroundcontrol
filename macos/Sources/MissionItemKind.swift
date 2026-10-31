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
        case "drop": return "shippingbox"
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

    var offersAny: Bool { all.contains { $0.enabled } }

    static func placed(_ kind: MissionItemKind, among items: [MissionItem]) -> MissionItem? {
        items.first { $0.kind == kind.id }
    }

    static func lastPlaced(_ kind: MissionItemKind, among items: [MissionItem]) -> MissionItem? {
        items.last.flatMap { $0.kind == kind.id ? $0 : nil }
    }
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

    func areaProperty(of item: MissionItem) -> String? {
        geometryProperty(of: item, shaped: "area")
    }

    func lineProperty(of item: MissionItem) -> String? {
        geometryProperty(of: item, shaped: "line")
    }

    private func geometryProperty(of item: MissionItem, shaped: String) -> String? {
        byId(item.kind).flatMap { $0.geometry == shaped ? $0.geometryProperty : nil }
    }

    func offers(pattern name: String) -> Bool {
        byComplexName(name)?.enabled ?? true
    }

    func refusal(pattern name: String) -> String? {
        byComplexName(name)?.disabledReason
    }

    static let refusedWithoutReason = "That item cannot go here."
}

enum InsertOutcome: Equatable {
    case inserted
    case insertDirectly(String)
    case refused(String)

    init(_ answer: [String: Any]) {
        if (answer["ok"] as? NSNumber)?.boolValue == true {
            self = .inserted
        } else if let name = answer["unknown"] as? String {
            self = .insertDirectly(name)
        } else {
            self = .refused((answer["reason"] as? String) ?? MissionKinds.refusedWithoutReason)
        }
    }
}

enum RemoveOutcome: Equatable {
    case removed
    case refused(String)

    static let refusedWithoutReason = "That item could not be removed."

    init(_ answer: [String: Any]) {
        guard (answer["ok"] as? NSNumber)?.boolValue == true else {
            self = .refused((answer["reason"] as? String).flatMap { $0.isEmpty ? nil : $0 }
                ?? RemoveOutcome.refusedWithoutReason)
            return
        }
        self = .removed
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
