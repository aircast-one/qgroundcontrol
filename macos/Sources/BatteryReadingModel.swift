import Foundation

struct BatteryReading: Equatable {
    let voltage: Double?
    let voltageText: String
    let currentText: String
    let percentText: String

    static let unavailable = BatteryReading(voltage: nil)
    static let unreported = "\u{2014}"

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
