import Foundation

enum Measure {
    static let defaultUnits = "m"
    static let unreported = "\u{2014}"

    // Two spellings of "nothing here", apart on purpose and until now held apart by nothing.
    // unreported is a DASH and belongs where the value is a MEASUREMENT sitting in a column of
    // numbers -- an altitude, a signal level, a duration -- where a word breaks the alignment the
    // column exists for. unread is a SENTENCE and belongs in a label/value row whose value is
    // arbitrary text, where a dash reads as the answer rather than as its absence. Both were
    // spelled at call sites in files swift-checks does not compile, and the two rows using the
    // sentence had each spelled it independently.
    static let unread = "Not reported"

    static func rowValue(_ value: String) -> String { value.isEmpty ? unread : value }

    // Asks the INPUT, never the printed text. The core serves the literal phrase "Not reported"
    // as camera storageText, so a row that decided its colour by comparing what it drew against
    // unread would grey out a real answer the vehicle gave.
    static func rowReported(_ value: String) -> Bool { !value.isEmpty }

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
