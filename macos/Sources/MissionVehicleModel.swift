struct MissionVehicle: Equatable {
    let firmware: String
    let type: String
    let multiRotor: Bool
    let vtol: Bool
    let apmFirmware: Bool

    static let unknown = MissionVehicle(firmware: "", type: "", multiRotor: false,
                                        vtol: false, apmFirmware: false)

    var showsCruiseSpeed: Bool { !multiRotor }
    var showsHoverSpeed: Bool { multiRotor || vtol }
    var isDescribed: Bool { !firmware.isEmpty || !type.isEmpty }
    var showsAnything: Bool { isDescribed || showsCruiseSpeed || showsHoverSpeed }

    var acceptsMissionStartSpeed: Bool { !apmFirmware && !vtol }
}
