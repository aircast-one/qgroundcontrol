import Foundation

enum AppUnits {
    private static var cache: [String: Measure] = [:]

    static let horizontal = "HorizontalDistance"
    static let vertical = "VerticalDistance"
    static let area = "Area"
    static let speed = "Speed"

    static func measure(_ kind: String) -> Measure {
        if let known = cache[kind] { return known }
        let strings = Bridge.group("units")
        let units = (strings["appSettings\(kind)UnitsString"] as? String) ?? ""
        guard !units.isEmpty else { return .metres }
        let built = Measure(units: units, factor: factor(kind))
        cache[kind] = built
        return built
    }

    private static func factor(_ kind: String) -> Double {
        let method = kind == area
            ? "squareMetersToAppSettingsAreaUnits"
            : kind == speed
                ? "metersSecondToAppSettingsSpeedUnits"
                : "metersToAppSettings\(kind)Units"
        return (Bridge.invoke("units.\(method)", [1.0])["result"] as? NSNumber)?.doubleValue ?? 1
    }
}
