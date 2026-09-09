import Foundation

struct SurveyStats: Equatable {
    let available: Bool
    let shotsText: String
    let intervalText: String
    let footprintText: String
    let tooFast: Bool
    let warning: String
    let areaText: String
    let distanceText: String

    static let none = SurveyStats()

    private init() {
        available = false
        shotsText = ""
        intervalText = ""
        footprintText = ""
        tooFast = false
        warning = ""
        areaText = ""
        distanceText = ""
    }

    init(_ json: [String: Any]) {
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        shotsText = text("shotsText")
        intervalText = text("intervalText")
        footprintText = text("footprintText")
        tooFast = (json["tooFast"] as? NSNumber)?.boolValue ?? false
        warning = text("warning")
        areaText = text("areaText")
        distanceText = text("distanceText")
    }

    var describes: Bool { available }
}
