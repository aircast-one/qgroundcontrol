import Foundation

struct BatteryReading: Equatable {
    let voltage: Double?
    let voltageText: String
    let currentText: String
    let percentText: String

    static let unavailable = BatteryReading(voltage: nil)
    static let unreported = "\u{2014}"

    // The Power page lists the vehicle's calibration parameters per pack - BATT_* and then
    // BATT2_* - so the live readings above them must be per pack too. Feeding that card from
    // packs.first put pack 1's voltage directly beneath pack 2's multipliers, and an operator
    // comparing a meter against the screen would have written the wrong BATT2_VOLT_MULT.
    // APMPowerComponent.qml binds its Battery 1 block to getFactGroup("battery0") and its
    // Battery 2 block to getFactGroup("battery1"), so upstream is per pack and this head was not.
    static func list(_ json: Any?) -> [BatteryReading] {
        ((json as? [Any]) ?? []).compactMap(BatteryReading.init)
    }

    static func cardTitle(_ index: Int, of count: Int) -> String {
        count > 1 ? "Battery \(index + 1)" : "Measured now"
    }

    init(voltage: Double?, voltageText: String = "", currentText: String = "",
         percentText: String = "") {
        self.voltage = voltage
        self.voltageText = BatteryReading.shown(voltageText)
        self.currentText = BatteryReading.shown(currentText)
        self.percentText = BatteryReading.shown(percentText)
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        let measure = (json["voltage"] as? NSNumber)?.doubleValue
        self.init(voltage: measure.filter(\.isFinite),
                  voltageText: (json["voltageText"] as? String) ?? "",
                  currentText: (json["currentText"] as? String) ?? "",
                  percentText: (json["percentText"] as? String) ?? "")
    }

    var available: Bool { voltage != nil }

    static func shown(_ spelled: String) -> String {
        spelled.isEmpty ? unreported : spelled
    }
}

private extension Optional where Wrapped == Double {
    func filter(_ keep: (Double) -> Bool) -> Double? {
        flatMap { keep($0) ? $0 : nil }
    }
}
