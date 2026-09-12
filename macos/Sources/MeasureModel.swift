import Foundation

enum Measure {
    static let defaultUnits = "m"
    static let unreported = "\u{2014}"
    static let wholeNumberFrom = 100.0

    static func pretty(_ units: String) -> String {
        units.replacingOccurrences(of: "^2", with: "\u{00B2}")
    }

    static func reading(_ value: Double?, _ units: String) -> String {
        guard let value else { return unreported }
        return format(value, units)
    }

    static func format(_ value: Double, _ units: String) -> String {
        guard value.isFinite else { return unreported }
        let digits = abs(value) >= wholeNumberFrom ? 0 : 1
        let number = settled(String(format: "%.\(digits)f", value))
        return units.isEmpty ? number : "\(number) \(pretty(units))"
    }

    static func settled(_ number: String) -> String {
        guard number.hasPrefix("-"),
              number.dropFirst().allSatisfy({ $0 == "0" || $0 == "." }) else { return number }
        return String(number.dropFirst())
    }
}
