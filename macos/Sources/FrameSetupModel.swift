import Foundation

struct FrameSetup: Equatable {
    // NOT the core's `vehicleType`, which is a wire token -- MultiRotor, VTOL, RoverBoat. This is
    // vehicleTypeString, already spelled for a person. Holding the text under the token's name
    // meant three spellings of one idea and a fourth thing sharing a name with a different served
    // field, so anything comparing this head's answer to the core's `vehicleType` would have seen
    // a text against a token and called it a disagreement.
    let reportedType: String
    // Absent where view.frame says null: the airframes QGC answers -1 for, and a submarine before
    // its parameters arrive. The summary said "6 motors" for that submarine off a default fact.
    let motorCount: Int?

    static let unknown = FrameSetup(reportedType: "", motorCount: nil)

    var known: Bool { !reportedType.isEmpty || counted > 0 }

    private var counted: Int { motorCount ?? 0 }

    var motorText: String {
        counted > 0 ? "\(counted) motor\(counted == 1 ? "" : "s")" : "—"
    }

    var vehicleTypeText: String { reportedType.isEmpty ? "—" : reportedType }

    static let undefinedFrameClass = "0"

    static func needsFrameClass(_ selectedRaw: String?) -> Bool {
        selectedRaw == undefinedFrameClass
    }
}
