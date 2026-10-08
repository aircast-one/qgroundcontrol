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

enum FactWrite {
    static func wholeNumberRefusal(_ entry: String, required: Bool, subject: String) -> String? {
        let typed = entry.trimmingCharacters(in: .whitespaces)
        guard required, !typed.isEmpty, Double(typed) != nil, Int(typed) == nil else { return nil }
        return "\(subject) takes a whole number."
    }

    static let readOnly = "That value is read-only, so it was not written."

    static func drawsBits(_ kind: SettingsControl.Kind, _ bits: [ControlBit]) -> Bool {
        kind == .bitmask && !bits.isEmpty
    }

    static func toggling(_ bit: ControlBit, on: Bool, within value: Int) -> Int {
        on ? value | bit.value : value & ~bit.value
    }
}

struct ControlBit: Identifiable, Equatable {
    let label: String
    let value: Int
    let set: Bool

    var id: Int { value }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let raw = json["raw"] as? String,
              let value = Int(raw), value != 0 else { return nil }
        self.value = value
        label = (json["label"] as? String) ?? ""
        set = (json["set"] as? NSNumber)?.boolValue ?? false
    }
}

struct SettingsControl: Identifiable, Equatable {
    enum Kind {
        case toggle
        case choice
        case bitmask
        case text
        case number
        case label
        case dialog
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "toggle": self = .toggle
            case "choice": self = .choice
            case "bitmask": self = .bitmask
            case "text": self = .text
            case "number": self = .number
            case "label": self = .label
            case "dialog": self = .dialog
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
    let enabled: Bool
    let disabledReason: String
    let restartNotices: [String]
    let options: [ControlOption]
    let bits: [ControlBit]
    let wholeNumbersOnly: Bool
    let minimum: Double?
    let maximum: Double?
    let dialog: String

    var id: String { path }

    var boolValue: Bool { (value as? NSNumber)?.boolValue ?? false }
    var intValue: Int { (value as? NSNumber)?.intValue ?? 0 }

    var restartNotice: String { restartNotices.joined(separator: SettingsControl.between) }

    static let between = " \u{00B7} "

    func rowDescription(label: String) -> String {
        let parts = [label.isEmpty ? "" : name, restartNotice,
                     refusal].filter { !$0.isEmpty }
        return parts.joined(separator: " \u{00B7} ")
    }

    var acceptsWrite: Bool { !readOnly && enabled }

    static let readOnlyNote = "Read-only"

    func subtitle(showingUnits: Bool) -> String {
        [showingUnits ? units : "", refusal].filter { !$0.isEmpty }
            .joined(separator: " \u{00B7} ")
    }

    var refusal: String {
        if readOnly { return SettingsControl.readOnlyNote }
        return enabled ? "" : disabledReason
    }

    var drawsBits: Bool { FactWrite.drawsBits(kind, bits) }

    func toggling(_ bit: ControlBit, on: Bool) -> Int {
        FactWrite.toggling(bit, on: on, within: intValue)
    }

    var parameterOptions: [ParameterOption] {
        options.map { ParameterOption(label: $0.label, raw: $0.raw) }
    }

    static let noChoice = -1

    var choiceIndex: Int {
        options.firstIndex { $0.raw == valueString } ?? SettingsControl.noChoice
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
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? true
        disabledReason = (json["disabledReason"] as? String) ?? ""
        restartNotices = ((json["restartNotices"] as? [Any]) ?? []).compactMap { $0 as? String }
        options = ((json["options"] as? [Any]) ?? []).compactMap(ControlOption.init)
        bits = ((json["bits"] as? [Any]) ?? []).compactMap(ControlBit.init)
        wholeNumbersOnly = (json["wholeNumbersOnly"] as? NSNumber)?.boolValue ?? false
        minimum = (json["minimum"] as? NSNumber)?.doubleValue
        maximum = (json["maximum"] as? NSNumber)?.doubleValue
        dialog = (json["dialog"] as? String) ?? ""
    }

    var offersManualEntry: Bool { false }

    func refusal(_ entry: String) -> String? {
        guard kind == .number else { return nil }
        if let refused = Measure.numberRefusal(entry) { return refused }
        if let refused = FactWrite.wholeNumberRefusal(entry, required: wholeNumbersOnly,
                                                      subject: label.isEmpty ? name : label) {
            return refused
        }
        guard let typed = Double(entry.trimmingCharacters(in: .whitespaces)) else { return nil }
        let under = minimum.map { typed < $0 } ?? false
        let over = maximum.map { typed > $0 } ?? false
        guard under || over else { return nil }
        return FactRange.sentence(label.isEmpty ? name : label,
                                  lowest: minimum.map(SettingsControl.spell),
                                  highest: maximum.map(SettingsControl.spell))
    }

    var valueHelp: String { kind == .text ? valueString : rangeHint }

    var rangeHint: String {
        guard kind == .number else { return "" }
        return FactRange.hint(lowest: minimum.map(SettingsControl.spell),
                              highest: maximum.map(SettingsControl.spell))
    }

    static func spell(_ limit: Double) -> String {
        limit == limit.rounded() && abs(limit) < 1e15
            ? String(Int(limit))
            : String(format: "%g", limit)
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
    static func worthRemembering(_ sections: [SettingsSection]) -> Bool {
        !sections.isEmpty
    }

    static func cacheSurvives(vehicle previous: Int?, now: Int?) -> Bool {
        previous != nil && previous == now
    }

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

    static func emptyText(search: String) -> String {
        search.isEmpty
            ? "This page has no editable settings."
            : "No setting matches “\(search)”."
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
    let showsPacketRadio: Bool

    var id: String { title }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let title = json["title"] as? String, !title.isEmpty else { return nil }
        self.title = title
        sections = SettingsSection.list(json["sections"])
        showsLinks = (json["showsLinks"] as? NSNumber)?.boolValue ?? false
        showsAbout = (json["showsAbout"] as? NSNumber)?.boolValue ?? false
        showsVideoSources = (json["showsVideoSources"] as? NSNumber)?.boolValue ?? false
        showsPacketRadio = (json["showsPacketRadio"] as? NSNumber)?.boolValue ?? false
    }

    static func list(_ json: Any?) -> [SettingsPage] {
        ((json as? [Any]) ?? []).compactMap(SettingsPage.init)
    }
}
