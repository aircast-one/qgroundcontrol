import Foundation

enum ItemCameraBridge {
    static func read(_ index: Int) -> JSON? { MapBridge.read("view.itemCamera(\(index))") }

    @discardableResult static func set(_ index: Int, _ member: String, _ value: Any) -> Bool {
        Qgc.set("plan.missionController.visualItems.\(index).cameraSection.\(member)", value)
    }

    @discardableResult static func chooseAction(_ index: Int, _ choice: Int) -> Bool {
        Qgc.set("plan.missionController.visualItems.\(index).cameraSection.cameraAction.enumIndex", choice)
    }
}

let VIDEO_VIEW_PATH = "view.video"

enum VideoBridge {
    static func read() -> JSON? { MapBridge.read(VIDEO_VIEW_PATH) }
}

func shotPoints(_ view: JSON?) -> [TrackPoint] {
    (view?["shotPoints"].arrayOrNil ?? []).compactMap { $0.object == nil ? nil : coordinate($0) }
}

struct CameraChoices: Equatable {
    var labels: [String]
    var chosen: Int
}

struct CameraExtras: Equatable {
    var intervalTime: Double?
    var intervalDistance: Double?
    var distanceUnits: String
    var modeSupported: Bool
    var commandsMode: Bool
    var mode: Int
    var commandsGimbal: Bool
    var pitch: Double
    var yaw: Double
    var pitchRange: ClosedRange<Double>? = nil
    var yawRange: ClosedRange<Double>? = nil
}

private func measured(_ view: JSON, _ key: String) -> Double? {
    guard view[key].object != nil, let value = view[key]["value"].double, !value.isNaN else { return nil }
    return value
}

private func userRange(_ view: JSON, _ key: String) -> ClosedRange<Double>? {
    let slider = view[key]["slider"]
    guard slider.object != nil else { return nil }
    let from = slider["from"].double(.nan)
    let to = slider["to"].double(.nan)
    return from.isNaN || to.isNaN || from == to ? nil : min(from, to)...max(from, to)
}

func cameraExtras(_ view: JSON?) -> CameraExtras? {
    guard let view, view["available"].bool else { return nil }
    let distanceUnits = view["intervalDistance"].object == nil ? "" : view["intervalDistance"]["units"].string
    return CameraExtras(
        intervalTime: measured(view, "intervalTime"),
        intervalDistance: measured(view, "intervalDistance"),
        distanceUnits: distanceUnits.ifBlank("m"),
        modeSupported: view["cameraModeSupported"].bool,
        commandsMode: view["commandsMode"].bool,
        mode: view["cameraMode"].object == nil ? 0 : view["cameraMode"]["choice"].int(0),
        commandsGimbal: view["commandsGimbal"].bool,
        pitch: measured(view, "gimbalPitch") ?? 0,
        yaw: measured(view, "gimbalYaw") ?? 0,
        pitchRange: userRange(view, "gimbalPitch"),
        yawRange: userRange(view, "gimbalYaw")
    )
}

func cameraChoices(_ view: JSON?) -> CameraChoices? {
    guard let view, view["available"].bool else { return nil }
    let measure = view["cameraAction"]
    guard measure.object != nil, let listed = measure["choices"].arrayOrNil else { return nil }
    let labels = listed.map(\.string)
    guard labels.contains(where: { !$0.isBlank }) else { return nil }
    return CameraChoices(labels: labels, chosen: measure["choice"].int(-1))
}

func measureText(_ view: JSON?, _ key: String) -> String {
    guard let measure = view?[key], measure.object != nil else { return "" }
    let text = measure["text"].string
    guard !text.isBlank else { return "" }
    let units = measure["units"].string
    return units.isBlank ? text : "\(text) \(units)"
}

func namedAction(_ text: String) -> String {
    text.isBlank || Double(text.trimmingCharacters(in: .whitespaces)) != nil ? "" : text
}

func actionLabel(_ view: JSON?) -> String {
    guard let choices = cameraChoices(view), choices.labels.indices.contains(choices.chosen) else {
        return namedAction(measureText(view, "cameraAction"))
    }
    return choices.labels[choices.chosen]
}

func itemCameraNote(_ view: JSON?) -> String? {
    guard let view, view["available"].bool, !view["note"].isNull else { return nil }
    return view["note"].string.isBlank ? nil : view["note"].string
}

func itemCameraTextBeside(_ view: JSON?, _ pickerLabel: String?) -> String? {
    itemCameraText(view).flatMap { $0 == pickerLabel ? nil : $0 }
}

func itemCameraText(_ view: JSON?) -> String? {
    guard let view, view["available"].bool else { return nil }
    let action = actionLabel(view)
    let angles = [measureText(view, "gimbalPitch"), measureText(view, "gimbalYaw")].filter { !$0.isBlank }.joined(separator: " / ")
    let gimbal = !view["commandsGimbal"].bool || angles.isBlank ? "" : "gimbal \(angles)"
    let text = [action, gimbal].filter { !$0.isBlank }.joined(separator: " · ")
    return text.isBlank ? nil : text
}
