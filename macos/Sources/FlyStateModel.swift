import Foundation

struct FlyState: Equatable {
    enum Kind: String, CaseIterable {
        case notConnected
        case contactLost
        case flying
        case landing
        case armed
        case disarmed
        case unknown
    }

    static let kinds = Kind.allCases.filter { $0 != .unknown }

    static let noVehicle = "Connect a vehicle to fly"
    static let unnamedState = "Vehicle state unknown"

    let connected: Bool
    let armed: Bool
    let contactLost: Bool
    let rcSupported: Bool
    let rcSignalText: String
    let kind: Kind
    let line: String
    let staleNotice: String
    let mode: String

    static let none = FlyState()

    private init() {
        connected = false
        armed = false
        contactLost = false
        rcSupported = false
        rcSignalText = ""
        kind = .notConnected
        line = ""
        staleNotice = ""
        mode = ""
    }

    init(_ json: [String: Any]) {
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        connected = flag("connected")
        armed = flag("armed")
        contactLost = flag("contactLost")
        rcSupported = flag("rcSupported")
        rcSignalText = (json["rcSignalText"] as? String) ?? ""
        kind = Kind(rawValue: (json["state"] as? String) ?? "") ?? .unknown
        line = (json["stateText"] as? String) ?? ""
        staleNotice = (json["staleNotice"] as? String) ?? ""
        mode = (json["mode"] as? String) ?? ""
    }

    var display: String {
        if kind == .notConnected { return FlyState.noVehicle }
        return line.isEmpty ? FlyState.unnamedState : line
    }

    var alarming: Bool { kind == .contactLost || kind == .unknown }

    var linkLevel: FlyTelemetry.Level { contactLost ? .critical : .good }
}
