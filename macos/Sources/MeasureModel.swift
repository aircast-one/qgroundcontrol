import Foundation

enum Measure {
    static let defaultUnits = "m"
    static let unreported = "\u{2014}"

    static func fieldText(_ value: Double?, _ decimals: Int) -> String {
        guard let value, value.isFinite else { return "" }
        return String(format: "%.\(decimals)f", value)
    }

    static func committed(_ draft: String, showing value: Double?, decimals: Int) -> Double? {
        let trimmed = draft.trimmingCharacters(in: .whitespaces)
        guard let typed = Double(trimmed), typed.isFinite else { return nil }
        guard trimmed != fieldText(value, decimals), typed != value else { return nil }
        return typed
    }

    static func settled(_ number: String) -> String {
        guard number.hasPrefix("-"),
              number.dropFirst().allSatisfy({ $0 == "0" || $0 == "." }) else { return number }
        return String(number.dropFirst())
    }
}
