import Foundation

let WAYPOINT_ALTITUDE_DECIMALS = 1
let RALLY_ALTITUDE_DECIMALS = 2
let SURFACE_DISTANCE_DECIMALS = 2
let SURFACE_DISTANCE_MINIMUM = 0.1

func typedNumber(_ text: String) -> Double? {
    let typed = text.trimmed
    guard typed.filter({ $0 == "," }).count <= 1, !(typed.contains(",") && typed.contains(".")) else { return nil }
    return Double(typed.replacingOccurrences(of: ",", with: ".")).flatMap { $0.isFinite ? $0 : nil }
}

func parsedAltitude(_ text: String) -> Double? { typedNumber(text) }

func parsedSurfaceDistance(_ text: String, _ metresPerShown: Double) -> Double? {
    typedNumber(text).flatMap { $0 * metresPerShown >= SURFACE_DISTANCE_MINIMUM - 1e-9 ? $0 : nil }
}

func metresPerUnit(_ units: String) -> Double { units.trimmed == "ft" ? 0.3048 : 1.0 }

func altitudeFieldText(_ altitude: Double, _ decimals: Int) -> String {
    guard !altitude.isNaN else { return "" }
    let shown = String(format: "%.\(decimals)f", altitude)
    let trimmed = shown.contains(".") ? shown.trimmingTrailing("0").trimmingTrailing(".") : shown
    return trimmed == "-0" ? "0" : trimmed
}

let LATITUDE_LIMIT = 90.0
let LONGITUDE_LIMIT = 180.0

func parsedCoordinate(_ text: String, _ limit: Double) -> Double? {
    typedNumber(text).flatMap { (-limit...limit).contains($0) ? $0 : nil }
}
