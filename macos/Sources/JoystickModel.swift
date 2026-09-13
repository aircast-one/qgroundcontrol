import Foundation

struct JoystickSetting: Equatable {
    let name: String
    let type: String
    let units: String
    let minimum: Double?
    let maximum: Double?
    let enumValues: [String]
    let defaultFrom: String

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        type = (json["type"] as? String) ?? ""
        units = (json["units"] as? String) ?? ""
        minimum = (json["min"] as? NSNumber)?.doubleValue
        maximum = (json["max"] as? NSNumber)?.doubleValue
        enumValues = ((json["enumValues"] as? [Any]) ?? []).compactMap { $0 as? String }
        defaultFrom = (json["defaultFrom"] as? String) ?? ""
    }

    // A setting the core gives no default for takes it from somewhere else at runtime --
    // transmitterMode reads support.defaultTransmitterMode. Rendering a made-up default would
    // show the operator a value the vehicle will not use.
    var defaultIsElsewhere: Bool { !defaultFrom.isEmpty }

    var bounded: Bool { minimum != nil && maximum != nil }
}

struct JoystickFunction: Equatable {
    let id: String
    let required: Bool
    let rcChannel: Int?

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = json["id"] as? String, !id.isEmpty else { return nil }
        self.id = id
        required = (json["required"] as? NSNumber)?.boolValue ?? false
        rcChannel = (json["rcChannel"] as? NSNumber)?.intValue
    }
}

struct JoystickAction: Equatable {
    let id: String
    let action: String
    let repeats: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = json["id"] as? String else { return nil }
        self.id = id
        action = (json["action"] as? String) ?? ""
        repeats = (json["repeat"] as? NSNumber)?.boolValue ?? false
    }
}

struct JoystickMapping: Equatable {
    let axisMinimum: Double
    let axisMaximum: Double
    let functions: [JoystickFunction]
    let actions: [JoystickAction]
    let settings: [JoystickSetting]
    let transmitterModes: [Int]

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        let range = (json["axisRange"] as? [String: Any]) ?? [:]
        axisMinimum = (range["min"] as? NSNumber)?.doubleValue ?? 0
        axisMaximum = (range["max"] as? NSNumber)?.doubleValue ?? 0
        functions = ((json["functions"] as? [Any]) ?? []).compactMap(JoystickFunction.init)
        actions = ((json["actions"] as? [Any]) ?? []).compactMap(JoystickAction.init)
        settings = ((json["settings"] as? [Any]) ?? []).compactMap(JoystickSetting.init)
        transmitterModes = ((json["transmitterModes"] as? [Any]) ?? []).compactMap {
            ($0 as? [String: Any]).flatMap { ($0["mode"] as? NSNumber)?.intValue }
        }
    }

    var required: [JoystickFunction] { functions.filter(\.required) }

    func setting(_ name: String) -> JoystickSetting? { settings.first { $0.name == name } }
}
