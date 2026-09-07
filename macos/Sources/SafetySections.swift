import Foundation

struct SetupSection: Identifiable {
    let title: String
    let note: String
    let parameters: [String]
    var id: String { title }

    static let safety: [SetupSection] = [
        SetupSection(
            title: "Failsafe",
            note: "What the vehicle does when it stops hearing from the transmitter or the ground station.",
            parameters: ["FS_THR_ENABLE", "FS_THR_VALUE", "FS_GCS_ENABLE"]),
        SetupSection(
            title: "Battery",
            note: "Thresholds and the action taken when the pack runs low.",
            parameters: ["BATT_MONITOR", "BATT_CAPACITY", "BATT_LOW_VOLT", "BATT_LOW_MAH",
                         "BATT_FS_LOW_ACT", "BATT_CRT_VOLT", "BATT_FS_CRT_ACT"]),
        SetupSection(
            title: "Return to Launch",
            note: "The path home when a failsafe or the operator triggers a return.",
            parameters: ["RTL_ALT", "RTL_ALT_FINAL", "RTL_LOIT_TIME", "LAND_SPEED"]),
        SetupSection(
            title: "Geofence",
            note: "The boundary the vehicle will not cross, and what it does at the edge.",
            parameters: ["FENCE_ENABLE", "FENCE_TYPE", "FENCE_ACTION",
                         "FENCE_ALT_MAX", "FENCE_RADIUS", "FENCE_MARGIN"]),
        SetupSection(
            title: "Arming",
            note: "Which pre-arm checks must pass before the vehicle will arm.",
            parameters: ["ARMING_CHECK"]),
    ]

    static let power: [SetupSection] = [
        SetupSection(
            title: "Battery 1",
            note: "How the pack is measured. Compare the readings above against a meter and correct the multipliers until they agree.",
            parameters: ["BATT_MONITOR", "BATT_CAPACITY", "BATT_VOLT_MULT", "BATT_AMP_PERVLT",
                         "BATT_AMP_OFFSET", "BATT_VOLT_PIN", "BATT_CURR_PIN", "BATT_ARM_VOLT"]),
        SetupSection(
            title: "Battery 2",
            note: "A second pack. The rest of its settings appear once a monitor is chosen.",
            parameters: ["BATT2_MONITOR", "BATT2_CAPACITY", "BATT2_VOLT_MULT", "BATT2_AMP_PERVLT",
                         "BATT2_AMP_OFFSET", "BATT2_VOLT_PIN", "BATT2_CURR_PIN", "BATT2_ARM_VOLT"]),
    ]

    static let frame: [SetupSection] = [
        SetupSection(
            title: "Airframe",
            note: "The class picks the layout, the type picks how its arms are oriented. A change to the class takes effect after the vehicle reboots.",
            parameters: ["FRAME_CLASS", "FRAME_TYPE"]),
    ]

    static func present(_ sections: [SetupSection], in available: Set<String>) -> [(section: SetupSection, names: [String])] {
        sections.compactMap { section in
            let names = section.parameters.filter(available.contains)
            return names.isEmpty ? nil : (section, names)
        }
    }
}
