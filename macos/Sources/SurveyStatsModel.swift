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

enum SurveyWatch {
    // A survey answers its area and its shot interval as soon as it has a polygon, and its shot
    // COUNT and flown DISTANCE only once the transects are computed -- which happens after the
    // read that drew the panel. Measured: the panel showed an em-dash for both while the core
    // answered 1043 shots over 7047 m, and it stayed that way until something forced a reload.
    // Those are the two numbers an operator sizes a battery and a card by.
    static let properties = ["cameraShots", "complexDistance"]

    static func signals(_ index: Int) -> [String] {
        properties.map { "plan.missionController.visualItems.\(index).\($0)" }
    }

    static func signals(survey index: Int?) -> [String] {
        index.map(signals) ?? []
    }
}
