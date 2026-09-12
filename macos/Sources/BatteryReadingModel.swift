import Foundation

struct BatteryReading: Equatable {
    let voltage: Double?
    let current: Double?
    let percent: Double?
    let spelledVoltage: String

    static let unavailable = BatteryReading(voltage: nil, current: nil, percent: nil)

    init(voltage: Double?, current: Double?, percent: Double?, spelledVoltage: String = "") {
        self.voltage = voltage
        self.current = current
        self.percent = percent
        self.spelledVoltage = spelledVoltage
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        func measure(_ name: String) -> Double? {
            guard let number = json[name] as? NSNumber, number.doubleValue.isFinite else {
                return nil
            }
            return number.doubleValue
        }
        self.init(voltage: measure("voltage"), current: measure("current"),
                  percent: measure("percent"),
                  spelledVoltage: (json["voltageText"] as? String) ?? "")
    }

    var available: Bool { voltage != nil }

    var voltageText: String {
        spelledVoltage.isEmpty ? BatteryReading.text(voltage, "%.2f V") : spelledVoltage
    }
    var currentText: String { BatteryReading.text(current, "%.2f A") }
    var percentText: String { BatteryReading.text(percent, "%.0f%%") }

    static func text(_ value: Double?, _ format: String) -> String {
        guard let value, value.isFinite else { return "\u{2014}" }
        return String(format: format, value)
    }
}
