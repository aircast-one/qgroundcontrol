import Foundation

struct FrameSetup: Equatable {
    let vehicleType: String
    let motorCount: Int

    static let unknown = FrameSetup(vehicleType: "", motorCount: 0)

    var known: Bool { !vehicleType.isEmpty || motorCount > 0 }

    var motorText: String {
        motorCount > 0 ? "\(motorCount) motor\(motorCount == 1 ? "" : "s")" : "—"
    }

    var vehicleTypeText: String { vehicleType.isEmpty ? "—" : vehicleType }

    static let undefinedFrameClass = "0"

    static func needsFrameClass(_ selectedRaw: String?) -> Bool {
        selectedRaw == undefinedFrameClass
    }
}
