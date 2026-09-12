import Foundation

struct Measure: Equatable {
    let units: String
    let factor: Double

    static let metres = Measure(units: "m", factor: 1)

    init(units: String, factor: Double) {
        self.units = units
        self.factor = factor.isFinite && factor > 0 ? factor : 1
    }

    func convert(_ metric: Double) -> Double { metric * factor }

    func text(_ metric: Double) -> String { Measure.format(convert(metric), units) }

    var suffix: String { Measure.pretty(units) }

    static func pretty(_ units: String) -> String {
        units.replacingOccurrences(of: "^2", with: "\u{00B2}")
    }

    static let wholeNumberFrom = 100.0

    static func reading(_ value: Double?, _ units: String) -> String {
        guard let value else { return "\u{2014}" }
        return format(value, units)
    }

    static func format(_ value: Double, _ units: String) -> String {
        guard value.isFinite else { return "\u{2014}" }
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
