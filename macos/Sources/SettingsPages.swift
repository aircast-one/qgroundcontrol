import Foundation

struct ControlOption: Identifiable, Equatable {
    let label: String
    let raw: String

    var id: String { raw }

    var writable: Any { Double(raw) ?? raw }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let label = json["label"] as? String,
              let raw = json["raw"] as? String else { return nil }
        self.label = label
        self.raw = raw
    }
}

struct SettingsControl: Identifiable, Equatable {
    enum Kind {
        case toggle
        case choice
        case text
        case number
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "toggle": self = .toggle
            case "choice": self = .choice
            case "text": self = .text
            case "number": self = .number
            default: self = .unknown
            }
        }
    }

    let path: String
    let name: String
    let label: String
    let kind: Kind
    let value: AnyHashable?
    let valueString: String
    let display: String
    let units: String
    let readOnly: Bool
    let rebootRequired: Bool
    let options: [ControlOption]
    let decimalPlaces: Int
    let minimum: Double?
    let maximum: Double?

    var id: String { path }

    var boolValue: Bool { (value as? NSNumber)?.boolValue ?? false }
    var intValue: Int { (value as? NSNumber)?.intValue ?? 0 }

    // The core folds QGC's two cases into one flag: ParameterEditorDialog.qml distinguishes
    // "Vehicle reboot required after change" from "Application restart required after change",
    // and view.control sends vehicleRebootRequired || qgcRebootRequired. This head can only say
    // which is true of both, so it says the plainer thing rather than guessing which one.
    static let restartNotice = "Restart required after a change"

    var restartNotice: String { rebootRequired ? SettingsControl.restartNotice : "" }

    // The row shows the setting's own name under its label; a setting that needs a restart says
    // so on the same line rather than in a place an operator has to go looking for.
    func rowDescription(label: String) -> String {
        let parts = [label.isEmpty ? "" : name, restartNotice].filter { !$0.isEmpty }
        return parts.joined(separator: " \u{00B7} ")
    }

    var parameterOptions: [ParameterOption] {
        options.map { ParameterOption(label: $0.label, raw: $0.raw) }
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let path = json["path"] as? String, !path.isEmpty,
              let control = json["control"] as? String else { return nil }
        self.path = path
        kind = Kind(control)
        name = (json["name"] as? String) ?? ""
        label = (json["label"] as? String) ?? ""
        value = json["value"] as? AnyHashable
        valueString = (json["valueString"] as? String) ?? ""
        display = (json["display"] as? String) ?? ""
        units = (json["units"] as? String) ?? ""
        readOnly = (json["readOnly"] as? NSNumber)?.boolValue ?? false
        rebootRequired = (json["rebootRequired"] as? NSNumber)?.boolValue ?? false
        options = ((json["options"] as? [Any]) ?? []).compactMap(ControlOption.init)
        decimalPlaces = (json["decimalPlaces"] as? NSNumber)?.intValue ?? 0
        minimum = (json["minimum"] as? NSNumber)?.doubleValue
        maximum = (json["maximum"] as? NSNumber)?.doubleValue
    }

    static func list(_ json: Any?) -> [SettingsControl] {
        ((json as? [Any]) ?? []).compactMap(SettingsControl.init)
    }
}

struct SettingsSubsection: Identifiable, Equatable {
    let title: String
    let controls: [SettingsControl]

    var id: String { title }

    init(title: String, controls: [SettingsControl]) {
        self.title = title
        self.controls = controls
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        title = (json["title"] as? String) ?? ""
        controls = SettingsControl.list(json["controls"])
    }
}

struct SettingsSection: Identifiable, Equatable {
    let title: String
    let group: String
    let path: String
    let note: String
    let subsections: [SettingsSubsection]

    var id: String { path.isEmpty ? title : path }

    var controls: [SettingsControl] { subsections.flatMap(\.controls) }

    var showsUnits: Bool { controls.contains { !$0.units.isEmpty } }

    init(title: String, group: String, path: String, note: String,
         subsections: [SettingsSubsection]) {
        self.title = title
        self.group = group
        self.path = path
        self.note = note
        self.subsections = subsections
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let title = json["title"] as? String else { return nil }
        self.title = title
        group = (json["group"] as? String) ?? ""
        path = (json["path"] as? String) ?? ""
        note = (json["note"] as? String) ?? ""
        let listed = ((json["subsections"] as? [Any]) ?? []).compactMap(SettingsSubsection.init)
        let direct = SettingsControl.list(json["controls"])
        subsections = listed.isEmpty && !direct.isEmpty
            ? [SettingsSubsection(title: "", controls: direct)]
            : listed
    }

    static func list(_ json: Any?) -> [SettingsSection] {
        ((json as? [Any]) ?? []).compactMap(SettingsSection.init)
    }
}

struct SettingsPage: Identifiable, Equatable {
    let title: String
    let sections: [SettingsSection]
    let showsLinks: Bool
    let showsAbout: Bool
    let showsVideoSources: Bool

    var id: String { title }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let title = json["title"] as? String, !title.isEmpty else { return nil }
        self.title = title
        sections = SettingsSection.list(json["sections"])
        showsLinks = (json["showsLinks"] as? NSNumber)?.boolValue ?? false
        showsAbout = (json["showsAbout"] as? NSNumber)?.boolValue ?? false
        showsVideoSources = (json["showsVideoSources"] as? NSNumber)?.boolValue ?? false
    }

    static func list(_ json: Any?) -> [SettingsPage] {
        ((json as? [Any]) ?? []).compactMap(SettingsPage.init)
    }
}
