import Foundation

let GUIDED_SPEED = "view.guidedSpeed"

struct GuidedSpeed: Equatable {
    var label: String
    var unit: String
    var command: String?
    var initial: Double?
    var minimum: Double?
    var maximum: Double?
    var sentence: String
    var targetMetersSecond: Double
}

func guidedSpeedPath(_ target: Double) -> String {
    "\(GUIDED_SPEED)(\(String(format: "%.2f", target)))"
}

private func numberOrNull(_ view: JSON, _ key: String) -> Double? {
    view[key].isNull ? nil : view[key].double.flatMap { $0.isNaN ? nil : $0 }
}

private func textOrNull(_ view: JSON, _ key: String) -> String? {
    view[key].isNull || view[key].string.isBlank ? nil : view[key].string
}

func guidedSpeed(_ view: JSON?) -> GuidedSpeed? {
    guard let view, view["available"].bool else { return nil }
    return GuidedSpeed(
        label: textOrNull(view, "label") ?? "Speed",
        unit: view["unit"].string,
        command: textOrNull(view, "command"),
        initial: numberOrNull(view, "initial"),
        minimum: numberOrNull(view, "minimum"),
        maximum: numberOrNull(view, "maximum"),
        sentence: view["sentence"].isNull ? "" : view["sentence"].string,
        targetMetersSecond: view["targetMetersSecond"].double(0)
    )
}
