import Foundation

struct VehicleWarning: Equatable {
    let noGpsLock: Bool
    let prearmError: String

    static let none = VehicleWarning(noGpsLock: false, prearmError: "")

    static let noGpsLockText = "No GPS lock for vehicle"

    var lines: [String] {
        var listed: [String] = []
        if noGpsLock { listed.append(VehicleWarning.noGpsLockText) }
        if !prearmError.isEmpty { listed.append(prearmError) }
        return listed
    }

    var showing: Bool { !lines.isEmpty }

    static func assess(connected: Bool, requiresGpsFix: Bool, hasCoordinate: Bool,
                       armed: Bool, prearmError: String, healthReportSupported: Bool) -> VehicleWarning {
        guard connected else { return .none }
        return VehicleWarning(
            noGpsLock: requiresGpsFix && !hasCoordinate,
            prearmError: !armed && !healthReportSupported ? prearmError : "")
    }
}
