import Foundation

struct BatteryReading: Equatable {
    let voltage: Double?
    let current: Double?
    let percent: Double?

    static let unavailable = BatteryReading(voltage: nil, current: nil, percent: nil)

    var available: Bool { voltage != nil }

    var voltageText: String { BatteryReading.text(voltage, "%.2f V") }
    var currentText: String { BatteryReading.text(current, "%.2f A") }
    var percentText: String { BatteryReading.text(percent, "%.0f%%") }

    static func text(_ value: Double?, _ format: String) -> String {
        guard let value, value.isFinite else { return "—" }
        return String(format: format, value)
    }
}
