import Foundation

let GUIDED_ALTITUDE = "view.guidedAltitude"

struct GuidedAltitude: Equatable {
    var available: Bool
    var label: String
    var unit: String
    var current: Double?
    var minimum: Double?
    var maximum: Double?
    var sentence: String
    var deltaMeters: Double
    var sends: Bool
    var targetMeters: Double? = nil
    var currentMeters: Double? = nil
}

func guidedAltitudePath(_ target: Double, pause: Bool = false) -> String {
    "\(GUIDED_ALTITUDE)(\(String(format: "%.2f", target))\(pause ? ",pause" : ""))"
}

private func numberOrNull(_ view: JSON, _ key: String) -> Double? {
    view[key].isNull ? nil : view[key].double.flatMap { $0.isNaN ? nil : $0 }
}

func guidedAltitude(_ view: JSON?) -> GuidedAltitude? {
    guard let view, view["available"].bool else { return nil }
    return GuidedAltitude(
        available: true,
        label: view["label"].string,
        unit: view["unit"].string,
        current: numberOrNull(view, "current"),
        minimum: numberOrNull(view, "minimum"),
        maximum: numberOrNull(view, "maximum"),
        sentence: view["sentence"].isNull ? "" : view["sentence"].string,
        deltaMeters: view["deltaMeters"].double(0),
        sends: view["sends"].bool,
        targetMeters: numberOrNull(view, "targetMeters"),
        currentMeters: numberOrNull(view, "currentMeters")
    )
}

func altitudeDelta(_ reading: GuidedAltitude) -> Double? { reading.sends ? reading.deltaMeters : nil }

func rangeLabel(_ minimum: Double?, _ maximum: Double?, _ unit: String) -> String? {
    guard let minimum, let maximum, maximum > minimum else { return nil }
    let show = { (value: Double) -> String in
        let text = String(format: "%.1f", value)
        return text == "-0.0" ? "0.0" : text
    }
    return ["\(show(minimum)) to \(show(maximum))", unit].filter { !$0.isBlank }.joined(separator: " ")
}
