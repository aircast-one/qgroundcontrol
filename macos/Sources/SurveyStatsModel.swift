import Foundation

struct SurveyStats: Equatable {
    let shots: Int
    let secondsBetweenShots: Double
    let areaSquareMetres: Double
    let footprintSide: Double
    let footprintFrontal: Double
    let minimumInterval: Double

    static let none = SurveyStats(shots: 0, secondsBetweenShots: 0, areaSquareMetres: 0,
                                  footprintSide: 0, footprintFrontal: 0, minimumInterval: 0)

    var describes: Bool { shots > 0 || areaSquareMetres > 0 }

    var shotsText: String { shots > 0 ? "\(shots)" : "\u{2014}" }

    var intervalText: String { SurveyStats.interval(secondsBetweenShots) }

    var areaText: String { SurveyStats.area(areaSquareMetres) }

    var footprintText: String {
        guard footprintSide > 0, footprintFrontal > 0 else { return "\u{2014}" }
        return String(format: "%.1f \u{00D7} %.1f m", footprintSide, footprintFrontal)
    }

    var tooFast: Bool {
        minimumInterval > 0 && secondsBetweenShots > 0 && secondsBetweenShots < minimumInterval
    }

    var warning: String {
        guard tooFast else { return "" }
        return String(format: "The camera needs %.2f s between shots but the survey asks for %.2f s.",
                      minimumInterval, secondsBetweenShots)
    }

    static func interval(_ seconds: Double) -> String {
        guard seconds.isFinite, seconds > 0 else { return "\u{2014}" }
        return String(format: "%.2f s", seconds)
    }

    static func area(_ squareMetres: Double) -> String {
        guard squareMetres.isFinite, squareMetres > 0 else { return "\u{2014}" }
        if squareMetres >= 1_000_000 {
            return String(format: "%.2f km\u{00B2}", squareMetres / 1_000_000)
        }
        if squareMetres >= 10_000 {
            return String(format: "%.1f ha", squareMetres / 10_000)
        }
        return String(format: "%.0f m\u{00B2}", squareMetres)
    }
}
