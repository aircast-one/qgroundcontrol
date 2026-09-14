import Foundation

struct MissionVehicle: Equatable {
    let firmware: String
    let type: String
    let multiRotor: Bool
    let vtol: Bool
    let apmFirmware: Bool

    static let unknown = MissionVehicle(firmware: "", type: "", multiRotor: false,
                                        vtol: false, apmFirmware: false)

    init(firmware: String, type: String, multiRotor: Bool, vtol: Bool, apmFirmware: Bool) {
        self.firmware = firmware
        self.type = type
        self.multiRotor = multiRotor
        self.vtol = vtol
        self.apmFirmware = apmFirmware
    }

    // planningFor is NULL until the controller resolves a vehicle to plan against, which is not
    // the same as a vehicle whose names happen to be empty -- one has not been chosen, the other
    // has and cannot describe itself. Returning nil keeps those apart at the call site.
    init?(planningFor json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        firmware = (json["firmware"] as? String) ?? ""
        type = (json["type"] as? String) ?? ""
        multiRotor = (json["multiRotor"] as? NSNumber)?.boolValue ?? false
        vtol = (json["vtol"] as? NSNumber)?.boolValue ?? false
        apmFirmware = (json["apmFirmware"] as? NSNumber)?.boolValue ?? false
    }

    // QGC's mavTypeToString is a hand-written map whose values are inconsistent in kind: short
    // names for most classes, but the two VTOL entries are the MAVLink enum DESCRIPTIONS, 78 and
    // 83 characters with a full stop in the middle. Only vtol is overridden. multiRotor's served
    // names -- Quadrotor, Hexarotor, Octorotor -- say more than a flag could, and the remaining
    // case covers fixed wing, rover, boat and submarine, which the flags cannot tell apart, so
    // naming it from them would assert. Every entry is tr()-wrapped, which is why this keys on
    // the served boolean and never on the string.
    var badge: String { vtol ? "VTOL" : type }

    var showsCruiseSpeed: Bool { !multiRotor }
    var showsHoverSpeed: Bool { multiRotor || vtol }
    var isDescribed: Bool { !firmware.isEmpty || !type.isEmpty }
    var showsAnything: Bool { isDescribed || showsCruiseSpeed || showsHoverSpeed }

    var acceptsMissionStartSpeed: Bool { !apmFirmware && !vtol }
}
