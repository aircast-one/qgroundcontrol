import Foundation

// The raw parameter list is not a mental model. QGC's safety page is hand-authored for
// exactly that reason, so this mirrors its grouping rather than exposing FS_*, BATT_*
// and FENCE_* as one alphabetical run.
//
// The names are ArduPilot's. A vehicle running different firmware simply will not have
// them, and a section with nothing present is dropped rather than shown empty -- which
// is also what makes this safe to extend with PX4 names later.
struct SafetySection: Identifiable {
    let title: String
    let note: String
    let parameters: [String]
    var id: String { title }

    static let all: [SafetySection] = [
        SafetySection(
            title: "Failsafe",
            note: "What the vehicle does when it stops hearing from the transmitter or the ground station.",
            parameters: ["FS_THR_ENABLE", "FS_THR_VALUE", "FS_GCS_ENABLE"]),
        SafetySection(
            title: "Battery",
            note: "Thresholds and the action taken when the pack runs low.",
            parameters: ["BATT_MONITOR", "BATT_CAPACITY", "BATT_LOW_VOLT", "BATT_LOW_MAH",
                         "BATT_FS_LOW_ACT", "BATT_CRT_VOLT", "BATT_FS_CRT_ACT"]),
        SafetySection(
            title: "Return to Launch",
            note: "The path home when a failsafe or the operator triggers a return.",
            parameters: ["RTL_ALT", "RTL_ALT_FINAL", "RTL_LOIT_TIME", "LAND_SPEED"]),
        SafetySection(
            title: "Geofence",
            note: "The boundary the vehicle will not cross, and what it does at the edge.",
            parameters: ["FENCE_ENABLE", "FENCE_TYPE", "FENCE_ACTION",
                         "FENCE_ALT_MAX", "FENCE_RADIUS", "FENCE_MARGIN"]),
        SafetySection(
            title: "Arming",
            note: "Which pre-arm checks must pass before the vehicle will arm.",
            parameters: ["ARMING_CHECK"]),
    ]

    // Only what this vehicle actually reports: a section listing parameters the
    // firmware does not have would imply settings that cannot be changed.
    static func present(in available: Set<String>) -> [(section: SafetySection, names: [String])] {
        all.compactMap { section in
            let names = section.parameters.filter(available.contains)
            return names.isEmpty ? nil : (section, names)
        }
    }
}
