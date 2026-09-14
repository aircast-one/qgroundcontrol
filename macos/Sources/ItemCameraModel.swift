import Foundation

struct ItemMeasure: Equatable {
    let text: String
    let units: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        text = (json["text"] as? String) ?? ""
        units = (json["units"] as? String) ?? ""
    }

    var spelled: String { units.isEmpty ? text : "\(text) \(units)" }
}

struct ItemCamera: Equatable {
    let available: Bool
    let commandsGimbal: Bool
    let gimbalPitch: ItemMeasure?
    let gimbalYaw: ItemMeasure?
    let cameraAction: ItemMeasure?

    static let none = ItemCamera()

    private init() {
        available = false
        commandsGimbal = false
        gimbalPitch = nil
        gimbalYaw = nil
        cameraAction = nil
    }

    init?(_ json: [String: Any]) {
        guard json["class"] as? String == "ItemCamera" else { return nil }
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        commandsGimbal = (json["commandsGimbal"] as? NSNumber)?.boolValue ?? false
        gimbalPitch = ItemMeasure(json["gimbalPitch"])
        gimbalYaw = ItemMeasure(json["gimbalYaw"])
        cameraAction = ItemMeasure(json["cameraAction"])
    }

    // The angles are Facts and a Fact always carries a number, so an item that never touches the
    // gimbal still reports a pitch and a yaw. `specifyGimbal` is the thing that says whether they
    // mean anything, and because it is a plain bool rather than a Fact it does not travel beside
    // them -- a head reading the angles alone is told the gimbal points somewhere for an item that
    // does not command it. The core already nulls them when it is false; this refuses to draw them
    // on `commandsGimbal` as well, so the rule is visible on the side that would show the number.
    var gimbalRows: [DetailRow] {
        guard available, commandsGimbal else { return [] }
        return [DetailRow(label: "Gimbal pitch", value: gimbalPitch?.spelled ?? ""),
                DetailRow(label: "Gimbal yaw", value: gimbalYaw?.spelled ?? "")]
            .filter { !$0.value.isEmpty }
    }

    // A camera action whose text is a bare number is the Fact's raw value with no enum label bound
    // to it -- "0.000" rather than "No change". That is a number about a menu, and showing it to an
    // operator is worse than showing nothing, because it looks like a setting they chose.
    static func labelled(_ text: String) -> Bool {
        !text.isEmpty && Double(text) == nil
    }

    var actionRow: DetailRow? {
        guard available, let cameraAction, ItemCamera.labelled(cameraAction.text) else { return nil }
        return DetailRow(label: "Camera", value: cameraAction.spelled)
    }

    var rows: [DetailRow] { (actionRow.map { [$0] } ?? []) + gimbalRows }
}
