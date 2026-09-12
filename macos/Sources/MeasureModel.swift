import Foundation

enum Measure {
    static let defaultUnits = "m"
    static let unreported = "\u{2014}"

    static func settled(_ number: String) -> String {
        guard number.hasPrefix("-"),
              number.dropFirst().allSatisfy({ $0 == "0" || $0 == "." }) else { return number }
        return String(number.dropFirst())
    }
}
