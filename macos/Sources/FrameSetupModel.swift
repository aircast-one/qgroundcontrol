import Foundation

struct FrameSetup: Equatable {
    let vehicleType: String
    // Absent where view.frame says null: the airframes QGC answers -1 for, and a submarine before
    // its parameters arrive. The summary said "6 motors" for that submarine off a default fact.
    let motorCount: Int?

    static let unknown = FrameSetup(vehicleType: "", motorCount: nil)

    var known: Bool { !vehicleType.isEmpty || counted > 0 }

    private var counted: Int { motorCount ?? 0 }

    var motorText: String {
        counted > 0 ? "\(counted) motor\(counted == 1 ? "" : "s")" : "—"
    }

    var vehicleTypeText: String { vehicleType.isEmpty ? "—" : vehicleType }

    static let undefinedFrameClass = "0"

    static func needsFrameClass(_ selectedRaw: String?) -> Bool {
        selectedRaw == undefinedFrameClass
    }
}
