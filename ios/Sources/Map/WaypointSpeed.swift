import Foundation

struct WaypointSpeed: Equatable {
    var specified: Bool
    var value: Double?
    var units: String
    var path: String
    var specifyPath: String
}

func waypointSpeed(_ view: JSON?) -> WaypointSpeed? {
    guard let section = view?["speedSection"], section.object != nil, section["available"].bool else { return nil }
    return WaypointSpeed(
        specified: section["specified"].bool,
        value: section["value"].isNull ? nil : section["value"].double(.nan),
        units: section["units"].string,
        path: section["path"].string,
        specifyPath: section["specifyPath"].string
    )
}

struct WaypointHold: Equatable {
    var seconds: Double
    var units: String
    var path: String
}

func waypointHold(_ view: JSON?) -> WaypointHold? {
    guard let hold = view?["hold"], hold.object != nil else { return nil }
    let made = WaypointHold(seconds: hold["value"].double(0), units: hold["units"].string, path: hold["path"].string)
    return made.path.isBlank ? nil : made
}
