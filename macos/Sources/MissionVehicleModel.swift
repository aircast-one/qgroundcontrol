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

    var showsCruiseSpeed: Bool { !multiRotor }
    var showsHoverSpeed: Bool { multiRotor || vtol }
    var isDescribed: Bool { !firmware.isEmpty || !type.isEmpty }
    var showsAnything: Bool { isDescribed || showsCruiseSpeed || showsHoverSpeed }

    var acceptsMissionStartSpeed: Bool { !apmFirmware && !vtol }
}
