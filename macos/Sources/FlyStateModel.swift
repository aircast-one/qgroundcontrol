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
    // NULLABLE because the core sends a verdict only while communicationLostEnabled is on.
    // flystate.rs served it as a plain bool and `flag()` defaulted a missing one to false, so an
    // unmonitored link arrived here as "not lost" and linkLevel drew it GREEN -- the head
    // asserting a link is fine on the strength of a check nobody is running.
    let contactLost: Bool?
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
        contactLost = nil
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
        contactLost = (json["contactLost"] as? NSNumber)?.boolValue
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

    // Three states, not two. A null is neither .good nor .critical: it is .unknown, which draws
    // in secondary the way the Traffic row does when traffic is off -- the established treatment
    // for a thing nobody is checking. Saying .good here would be the same defect as the served
    // false it replaces, only written in Swift.
    var linkLevel: FlyTelemetry.Level {
        switch contactLost {
        case .some(true): return .critical
        case .some(false): return .good
        case nil: return .unknown
        }
    }
}
