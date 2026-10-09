import Foundation

let GUIDED_TAKEOFF = "view.guidedTakeoff"

struct GuidedTakeoff: Equatable {
    var label: String
    var unit: String
    var initial: Double?
    var minimum: Double?
    var maximum: Double?
    var sentence: String
    var targetMeters: Double
}

func guidedTakeoffPath(_ target: Double) -> String {
    "\(GUIDED_TAKEOFF)(\(String(format: "%.2f", target)))"
}

func guidedTakeoff(_ view: JSON?) -> GuidedTakeoff? {
    guard let view, view["available"].bool else { return nil }
    return GuidedTakeoff(
        label: view["label"].string,
        unit: view["unit"].string,
        initial: view["initial"].nonNanDouble,
        minimum: view["minimum"].nonNanDouble,
        maximum: view["maximum"].nonNanDouble,
        sentence: view["sentence"].isNull ? "" : view["sentence"].string,
        targetMeters: view["targetMeters"].double(0)
    )
}

func takeoffRangeUsable(_ takeoff: GuidedTakeoff?) -> Bool {
    guard let minimum = takeoff?.minimum, let maximum = takeoff?.maximum else { return false }
    return maximum > minimum
}
