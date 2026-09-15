import Foundation

struct FleetVehicle: Equatable {
    let id: Int
    let name: String
    let type: String
    let firmware: String
    let link: String
    let contactLost: Bool?
    let active: Bool
    let armed: Bool
    let flying: Bool
    let flightMode: String
    let placed: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = (json["id"] as? NSNumber)?.intValue else { return nil }
        self.id = id
        name = (json["name"] as? String) ?? ""
        type = (json["type"] as? String) ?? ""
        firmware = (json["firmware"] as? String) ?? ""
        link = (json["link"] as? String) ?? ""
        contactLost = (json["contactLost"] as? NSNumber)?.boolValue
        active = (json["active"] as? NSNumber)?.boolValue ?? false
        armed = (json["armed"] as? NSNumber)?.boolValue ?? false
        flying = (json["flying"] as? NSNumber)?.boolValue ?? false
        flightMode = (json["flightMode"] as? String) ?? ""
        placed = json["coordinate"] is [String: Any]
    }

    // contactLost is nullable because the core sends it only when communicationLostEnabled is on.
    // Null is NOT "in contact" -- it is a vehicle nobody is watching for silence, which is a
    // different thing to report than one that is currently being heard from. The same rule the
    // obstacle panel needed for `stale`.
    var contactKnown: Bool { contactLost != nil }

    var lost: Bool { contactLost == true }

    // The state line says the strongest true thing and stops. Flying outranks armed because an
    // armed vehicle on the ground and one in the air are not equally urgent, and a lost vehicle
    // outranks both -- what it was doing when contact went is no longer a fact about now.
    var stateText: String {
        if lost { return "Contact lost" }
        if flying { return flightMode.isEmpty ? "Flying" : "Flying \u{00B7} \(flightMode)" }
        if armed { return flightMode.isEmpty ? "Armed" : "Armed \u{00B7} \(flightMode)" }
        return flightMode.isEmpty ? "Idle" : flightMode
    }

    var listTitle: String { active ? "\(name) \u{00B7} active" : name }

    var level: FlyTelemetry.Level {
        if lost { return .warning }
        if !contactKnown { return .unknown }
        return flying || armed ? .caution : .good
    }
}

struct Fleet: Equatable {
    let count: Int
    let activeId: Int?
    let ambiguous: Bool
    let vehicles: [FleetVehicle]

    static let none = Fleet()

    private init() {
        count = 0
        activeId = nil
        ambiguous = false
        vehicles = []
    }

    init?(_ json: [String: Any]) {
        guard json["class"] as? String == "Vehicles" else { return nil }
        count = (json["count"] as? NSNumber)?.intValue ?? 0
        activeId = (json["activeId"] as? NSNumber)?.intValue
        ambiguous = (json["ambiguous"] as? NSNumber)?.boolValue ?? false
        vehicles = ((json["vehicles"] as? [Any]) ?? []).compactMap(FleetVehicle.init)
    }

    // One vehicle needs no list: everything already on the Fly view is about it, and a panel
    // repeating that is the noise an operator learns to skip. The list earns its place exactly when
    // the rest of the screen has become ambiguous about which vehicle it is describing, which is
    // the question `ambiguous` answers.
    var worthShowing: Bool { ambiguous }
}
