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
    // QGC refuses this at the type conversion, BEFORE the range check: ParameterEditorDialog passes
    // the TEXT to Fact::validate and QVariant("3.7").toInt() fails on a string carrying a decimal
    // point. This head sent a Double instead, so Fact::setRawValue ran convertAndValidateRaw with
    // convertOnly -- the range check skipped entirely -- and QVariant(3.7).toInt() answered 3 with
    // convertOk true. The field showed 3.7, the vehicle got 3, and the bridge answered ok:true
    // because the setter had run. Recorded as unfixable in dd04e0470 because no head was told which
    // facts are integers; typeIsInteger and wholeNumbersOnly are that answer.
    // Int(entry) is the same test QVariant makes: "5" converts, "5.0" does not, and QGC refuses
    // "5.0" on an integer fact too. The order matches convertAndValidateCooked -- parseable, then
    // type, then range -- so an out-of-range fraction is told it is not a whole number first.
    static func wholeNumberRefusal(_ entry: String, required: Bool, subject: String) -> String? {
        let typed = entry.trimmingCharacters(in: .whitespaces)
        guard required, !typed.isEmpty, Double(typed) != nil, Int(typed) == nil else { return nil }
        return "\(subject) takes a whole number."
    }

    static let readOnly = "That value is read-only, so it was not written."

    // Both a vehicle parameter and an app setting can be a bitmask, and both must answer these the
    // same way, so the rules live once here rather than as twins on the two models. The kind is the
    // producer's own word: control.rs resolves a fact carrying BOTH enum labels and bitmask labels
    // to "choice", as the Qt editor does, so a row must not decide that for itself by asking which
    // list is empty.
    static func drawsBits(_ kind: SettingsControl.Kind, _ bits: [ControlBit]) -> Bool {
        kind == .bitmask && !bits.isEmpty
    }

    // Toggling one bit leaves every OTHER bit of the stored value alone, including bits the metadata
    // never named. Firmware sets bits the ground station has not heard of, and rebuilding the value
    // from the declared bits alone would silently clear them.
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
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "toggle": self = .toggle
            case "choice": self = .choice
            case "bitmask": self = .bitmask
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
    let restartNotices: [String]
    let options: [ControlOption]
    let bits: [ControlBit]
    let wholeNumbersOnly: Bool
    let minimum: Double?
    let maximum: Double?

    var id: String { path }

    var boolValue: Bool { (value as? NSNumber)?.boolValue ?? false }
    var intValue: Int { (value as? NSNumber)?.intValue ?? 0 }

    var restartNotice: String { restartNotices.joined(separator: SettingsControl.between) }

    static let between = " \u{00B7} "

    // The row shows the setting's own name under its label; a setting that needs a restart says
    // so on the same line rather than in a place an operator has to go looking for.
    func rowDescription(label: String) -> String {
        let parts = [label.isEmpty ? "" : name, restartNotice].filter { !$0.isEmpty }
        return parts.joined(separator: " \u{00B7} ")
    }

    var drawsBits: Bool { FactWrite.drawsBits(kind, bits) }

    func toggling(_ bit: ControlBit, on: Bool) -> Int {
        FactWrite.toggling(bit, on: on, within: intValue)
    }

    var parameterOptions: [ParameterOption] {
        options.map { ParameterOption(label: $0.label, raw: $0.raw) }
    }

    // A stored value matching no offered option must not resolve to the FIRST one. The picker
    // would assert the setting is something it is not, and a touch anywhere near it would write
    // that wrong value back as though the operator had chosen it. This index matches no tag, so
    // the control draws empty and says nothing instead of something false.
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
        restartNotices = ((json["restartNotices"] as? [Any]) ?? []).compactMap { $0 as? String }
        options = ((json["options"] as? [Any]) ?? []).compactMap(ControlOption.init)
        bits = ((json["bits"] as? [Any]) ?? []).compactMap(ControlBit.init)
        wholeNumbersOnly = (json["wholeNumbersOnly"] as? NSNumber)?.boolValue ?? false
        minimum = (json["minimum"] as? NSNumber)?.doubleValue
        maximum = (json["maximum"] as? NSNumber)?.doubleValue
    }

    // A RANGE CHECK THAT CANNOT PARSE ITS INPUT USED TO RETURN nil -- no objection -- and the
    // caller then wrote `Double(value) ?? value`, putting the raw STRING on the vehicle. So the
    // one guard standing between an operator and a bad write was silent on exactly the entry
    // that needed refusing. A comma is the common way in; anything unparseable is the class.
    // An app setting's choices come from QGC's own JSON and are exhaustive by construction -- there
    // is no firmware shipping a language or a map provider the list has not heard of. So this is
    // false where Parameter's is true, and the two are separate properties rather than one rule with
    // a flag, because the reason they differ is about the PRODUCER of the list and not about the row.
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

    // A settings value is drawn in a fixed, right-aligned field and a comma-separated list
    // overflows it -- Flight Modes shows "Acro,Circle,Drift,Sport,Flip,Bra...". Reading the rest
    // means clicking INTO the field, which is an editable control: entering it to read is how you
    // write what you came to check. The help is the one place the value can be read without
    // touching the thing that sets it.
    //
    // Unconditional for a text fact rather than keyed on a length, because the row cannot know its
    // own rendered width and so cannot know whether THIS value was cut; a threshold would be a
    // number with nothing behind it. A number keeps its range hint instead -- those do not
    // overflow, and the band is the more useful thing to find under the pointer.
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
