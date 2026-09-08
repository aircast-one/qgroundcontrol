import Foundation

struct GuidedRange: Equatable {
    let label: String
    let unit: String
    let minimum: Double
    let maximum: Double
    let initial: Double

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              (json["available"] as? NSNumber)?.boolValue == true,
              let minimum = (json["minimum"] as? NSNumber)?.doubleValue,
              let maximum = (json["maximum"] as? NSNumber)?.doubleValue,
              minimum.isFinite, maximum.isFinite, maximum > minimum else { return nil }
        let start = (json["initial"] as? NSNumber)?.doubleValue
            ?? (json["current"] as? NSNumber)?.doubleValue
            ?? minimum
        label = (json["label"] as? String) ?? ""
        unit = (json["unit"] as? String) ?? ""
        self.minimum = minimum
        self.maximum = maximum
        initial = Swift.min(Swift.max(start, minimum), maximum)
    }

    func clamped(_ value: Double) -> Double {
        Swift.min(Swift.max(value, minimum), maximum)
    }

    func text(_ value: Double) -> String {
        let shown = clamped(value)
        let digits = abs(shown) < 10 ? 1 : 0
        return unit.isEmpty
            ? String(format: "%.\(digits)f", shown)
            : String(format: "%.\(digits)f %@", shown, unit)
    }
}
