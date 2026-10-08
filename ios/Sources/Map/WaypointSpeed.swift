import SwiftUI

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

enum SpeedEntry: Equatable {
    case Clear
    case Set(value: Double)
    case Invalid
}

func speedEntry(_ text: String) -> SpeedEntry {
    guard !text.isBlank else { return .Clear }
    return typedNumber(text).flatMap { $0 > 0 ? SpeedEntry.Set(value: $0) : nil } ?? .Invalid
}

func speedFieldText(_ speed: WaypointSpeed) -> String {
    speed.specified ? speed.value.map(plainSpeed) ?? "" : ""
}

private func plainSpeed(_ value: Double) -> String {
    value == value.rounded(.down) && abs(value) < 1e18 ? String(Int64(value)) : String(value)
}

private let SPEED_FIELD_WIDTH: CGFloat = 120

struct WaypointSpeedField: View {
    let index: Int
    let onWrite: (String, @escaping () -> Bool) -> Void
    let onRefused: (String) -> Void
    @MapPath private var json: JSON?
    @State private var typed = ""

    init(index: Int, onWrite: @escaping (String, @escaping () -> Bool) -> Void, onRefused: @escaping (String) -> Void) {
        self.index = index
        self.onWrite = onWrite
        self.onRefused = onRefused
        _json = MapPath("view.itemFacts(\(index))")
    }

    var body: some View {
        if let speed = waypointSpeed(json) {
            PlanTextField(
                label: ["Speed", speed.units].filter { !$0.isBlank }.joined(separator: " "),
                text: $typed,
                placeholder: "Auto",
                width: SPEED_FIELD_WIDTH
            ) {
                switch speedEntry(typed) {
                case .Clear:
                    onWrite("Clearing the speed") { setOk(speed.specifyPath, false) }
                case .Set(let value):
                    onWrite("Setting the speed") { setOk(speed.specifyPath, true) && setOk(speed.path, value) }
                case .Invalid:
                    onRefused("Not a speed")
                }
            }
            .onChange(of: "\(index):\(speed.specified):\(String(describing: speed.value))", initial: true) {
                typed = speedFieldText(speed)
            }
        }
    }
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

func holdEntry(_ text: String) -> Double? {
    typedNumber(text.ifBlank("0")).flatMap { $0 >= 0 ? $0 : nil }
}

private let HOLD_FIELD_WIDTH: CGFloat = 110

struct WaypointHoldField: View {
    let index: Int
    let onWrite: (String, @escaping () -> Bool) -> Void
    let onRefused: (String) -> Void
    @MapPath private var json: JSON?
    @State private var typed = ""

    init(index: Int, onWrite: @escaping (String, @escaping () -> Bool) -> Void, onRefused: @escaping (String) -> Void) {
        self.index = index
        self.onWrite = onWrite
        self.onRefused = onRefused
        _json = MapPath("view.itemFacts(\(index))")
    }

    var body: some View {
        if let hold = waypointHold(json) {
            PlanTextField(label: "Hold", text: $typed, suffix: hold.units, width: HOLD_FIELD_WIDTH) {
                if let seconds = holdEntry(typed) {
                    onWrite("Setting the hold") { setOk(hold.path, seconds) }
                } else {
                    onRefused("Not a hold time")
                }
            }
            .onChange(of: "\(index):\(hold.seconds)", initial: true) {
                typed = plainSpeed(hold.seconds)
            }
        }
    }
}
