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

    static func decimals(matching served: String) -> Int? {
        guard let dot = served.firstIndex(of: ".") else {
            return served.contains(where: \.isNumber) ? 0 : nil
        }
        let after = served[served.index(after: dot)...].prefix { $0.isNumber }
        return after.isEmpty ? nil : after.count
    }

    // Swift's Double(String) is C-locale in every locale, so "12,5" is nil wherever it is typed
    // and the rig being English never hid this. What the three writers did with that nil differed:
    // two returned in silence and the third wrote the raw STRING onward.
    //
    // The one place that already "solved" it carried the worst defect. FactControl replaced every
    // comma with a full stop, so an operator typing 1,500 got Double("1.500") -- ONE POINT FIVE,
    // a silent thousand-fold error in a field that sets altitudes and speeds.
    //
    // A comma is genuinely ambiguous: "12,500" is twelve and a half to a German operator and
    // twelve thousand five hundred to an English one, and nothing in the string says which. So
    // this refuses rather than choosing, because guessing is how 1,500 became 1.5.
    static let commaAdvice = "Use a full stop for the decimal point, and no thousands separator."

    static func numberRefusal(_ entry: String) -> String? {
        let trimmed = entry.trimmingCharacters(in: .whitespaces)
        if trimmed.contains(",") { return Measure.commaAdvice }
        guard Double(trimmed) != nil else { return "\(trimmed) is not a number." }
        return nil
    }

    static func settled(_ number: String) -> String {
        guard number.hasPrefix("-"),
              number.dropFirst().allSatisfy({ $0 == "0" || $0 == "." }) else { return number }
        return String(number.dropFirst())
    }
}
