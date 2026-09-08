import Foundation

struct SurveyStats: Equatable {
    let shots: Int
    let secondsBetweenShots: Double
    let areaSquareMetres: Double
    let distanceMetres: Double
    let footprintSide: Double
    let footprintFrontal: Double
    let footprintUnits: String
    let minimumInterval: Double
    let areaMeasure: Measure
    let distanceMeasure: Measure

    static let none = SurveyStats(shots: 0, secondsBetweenShots: 0, areaSquareMetres: 0,
                                  distanceMetres: 0, footprintSide: 0, footprintFrontal: 0,
                                  footprintUnits: "m", minimumInterval: 0,
                                  areaMeasure: .squareMetres, distanceMeasure: .metres)

    var describes: Bool { shots > 0 || areaSquareMetres > 0 }

    var shotsText: String { shots > 0 ? "\(shots)" : "\u{2014}" }

    var intervalText: String { SurveyStats.interval(secondsBetweenShots) }

    var areaText: String {
        areaSquareMetres > 0 ? areaMeasure.text(areaSquareMetres) : "\u{2014}"
    }

    var distanceText: String {
        distanceMetres > 0 ? distanceMeasure.text(distanceMetres) : "\u{2014}"
    }

    var footprintText: String {
        guard footprintSide > 0, footprintFrontal > 0 else { return "\u{2014}" }
        return String(format: "%.1f \u{00D7} %.1f %@", footprintSide, footprintFrontal,
                      Measure.pretty(footprintUnits))
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
        return String(format: "%.1f s", seconds)
    }
}
