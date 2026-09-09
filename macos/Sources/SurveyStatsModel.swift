import Foundation

struct SurveyStats: Equatable {
    let available: Bool
    let shotsText: String
    let intervalText: String
    let footprintText: String
    let tooFast: Bool
    let warning: String
    let areaSquareMetres: Double
    let distanceMetres: Double
    var areaMeasure = Measure.squareMetres
    var distanceMeasure = Measure.metres

    static let none = SurveyStats()

    private init() {
        available = false
        shotsText = ""
        intervalText = ""
        footprintText = ""
        tooFast = false
        warning = ""
        areaSquareMetres = 0
        distanceMetres = 0
    }

    init(_ json: [String: Any]) {
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        shotsText = text("shotsText")
        intervalText = text("intervalText")
        footprintText = text("footprintText")
        tooFast = (json["tooFast"] as? NSNumber)?.boolValue ?? false
        warning = text("warning")
        areaSquareMetres = (json["areaSquareMetres"] as? NSNumber)?.doubleValue ?? 0
        distanceMetres = (json["distanceMetres"] as? NSNumber)?.doubleValue ?? 0
    }

    var describes: Bool { available }

    // QGC's formatMeasure takes one decimal below a hundred and none above, and writes the
    // area unit as m² rather than the m^2 the settings string carries. The core does
    // neither yet, so these two texts stay here rather than regress the display.
    var areaText: String {
        areaSquareMetres > 0 ? areaMeasure.text(areaSquareMetres) : "\u{2014}"
    }

    var distanceText: String {
        distanceMetres > 0 ? distanceMeasure.text(distanceMetres) : "\u{2014}"
    }
}
