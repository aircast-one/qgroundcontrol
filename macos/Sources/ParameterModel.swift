import Foundation

struct ParameterOption: Identifiable, Equatable {
    let label: String
    let raw: String

    var id: String { raw }

    init(label: String, raw: String) {
        self.label = label
        self.raw = raw
    }

    // view.control serves options already paired and already filtered: control.rs drops the
    // synthetic "Unknown: N" entry at the producer, and zips labels to raws only when the two
    // lists are the same length. Both were rules in this file and both were defects first.
    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let label = json["label"] as? String,
              let raw = json["raw"] as? String else { return nil }
        self.label = label
        self.raw = raw
    }

    static let manualEntryHelp = "Set a value the list does not offer. Firmware accepts values "
        + "before the metadata names them."
}

struct Parameter: Identifiable {
    let name: String
    let componentId: Int
    let value: String
    let units: String
    let description: String
    let options: [ParameterOption]
    let range: FactRange
    // The producer's own word for what this row is, rather than four booleans derived from which
    // lists came back empty. control.rs resolves a fact carrying both enum labels and bitmask labels
    // to a choice exactly as the Qt editor does, and that judgement is not one a row should re-make.
    let kind: SettingsControl.Kind
    // The value a bit is toggled within. Read from the fact's own number and never parsed back out
    // of the display string: display is cooked -- it is the enum LABEL wherever there is one.
    let numericValue: Int
    // Composed by the core as {label, raw, set}, so nothing here derives set from value & bit.
    let bits: [ControlBit]
    // typeIsInteger was added to Fact and to kFactProperties for exactly this, and control.rs turns
    // the same answer into wholeNumbersOnly. NOT decimalPlaces: a real-typed fact declaring zero
    // decimals is what you write for a percentage, and keying on it would refuse fractions the
    // vehicle accepts -- the head-side fix rejected in dd04e0470.
    let wholeNumbersOnly: Bool

    var id: String { "\(componentId)/\(name)" }
    var path: String { "vehicle.parameterManager.getParameter(\(componentId),\(name))" }
    // The nested form parses -- measured on the rig -- and the view read is FASTER than the raw
    // fact it replaces, 3.87ms against 5.78ms, because a raw fact drags all thirty allowlisted
    // properties including two long parallel bitmask arrays.
    var controlPath: String { "view.control(\(path))" }

    var group: String {
        guard let underscore = name.firstIndex(of: "_") else { return name }
        return String(name[name.startIndex..<underscore])
    }

    init(name: String, componentId: Int, json: [String: Any]) {
        self.name = name
        self.componentId = componentId
        kind = SettingsControl.Kind(json["control"] as? String)
        numericValue = (json["value"] as? NSNumber)?.intValue ?? 0
        wholeNumbersOnly = (json["wholeNumbersOnly"] as? NSNumber)?.boolValue ?? false
        units = (json["units"] as? String) ?? ""
        // label is the fact's shortDescription where it has one and humanise(name) where it does
        // not. humanise leaves an all-caps name alone -- capitalise only uppercases a first
        // character that is already uppercase -- so a parameter with no metadata still reads as its
        // own name here. Checked in label.rs rather than assumed, because a mangled name in the one
        // screen somebody opens to look a parameter up would be worse than no migration.
        description = (json["label"] as? String) ?? ""
        range = FactRange(control: json, title: name)
        options = ((json["options"] as? [Any]) ?? []).compactMap(ParameterOption.init)
        bits = ((json["bits"] as? [Any]) ?? []).compactMap(ControlBit.init)
        value = (json["display"] as? String) ?? ""
    }
}

extension Parameter {
    // The numeric rules apply to numeric parameters only. A string parameter legitimately holds
    // a comma, a letter or anything else, and refusing it would break a write that is correct --
    // worse than the hole it closes.
    func refusal(_ entry: String) -> String? {
        guard !isString else { return nil }
        if let refused = Measure.numberRefusal(entry) { return refused }
        if let refused = FactWrite.wholeNumberRefusal(entry, required: wholeNumbersOnly,
                                                      subject: name) {
            return refused
        }
        return range.refusal(entry)
    }

    // A vehicle parameter's enum list is not the set of values the firmware accepts. ArduPilot ships
    // values ahead of the metadata that names them, which is why QGC's own editor carries an escape:
    // ParameterEditorDialog.qml:240's manualEntry checkbox swaps the combo for a free-text field.
    // Without it this is the one screen where "it is not in the list" is the answer somebody came
    // for and the screen cannot give it -- the same shape as b09ed8fee. A typed value is bounded by
    // refusal above, exactly as a numeric parameter's is; the escape widens what can be asked for,
    // not what can be written unchecked. A parameter with no options already gets the field.
    var offersManualEntry: Bool { !options.isEmpty }

    var isString: Bool { kind == .text }

    // ARMING_CHECK read "82" on a setup page and in the parameter list: a number an operator has to
    // decompose in their head to find out that the compass check is off. The settings window has
    // drawn named toggles since the bitmask control existed; the two screens that show vehicle
    // parameters went through ParameterRow, which had no branch for them and fell through to the
    // plain value field.
    var drawsBits: Bool { FactWrite.drawsBits(kind, bits) }

    // A 328-row browser cannot give one parameter nineteen rows of checkboxes, which is what the
    // first version of this did: ARMING_CHECK filled the viewport and the other 327 parameters went
    // below the fold. The names are the answer somebody scrolls here for, so the list states them
    // and keeps the numeric field for the write. No bit set is not nothing set -- it is 0, and the
    // served display says so rather than leaving the slot blank.
    var bitSummary: String {
        let on = bits.filter(\.set).map(\.label)
        return on.isEmpty ? value : on.joined(separator: ", ")
    }

    // Returns the text the row commits, so the write goes through the one path that checks refusal
    // and re-reads the row, rather than a second writer that would have to repeat both.
    func toggling(_ bit: ControlBit, on: Bool) -> String {
        String(FactWrite.toggling(bit, on: on, within: numericValue))
    }

    // The row draws description as its title and this underneath. Before the migration description
    // was the raw shortDescription and empty when a fact had none, so the row fell back to the name
    // and showed nothing here. The core's label never comes back empty -- it answers the name
    // itself in that case -- so the old `description.isEmpty` test would print the name twice.
    var rowDetail: String { description == name ? "" : name }

    static func rawText(_ value: Any) -> String {
        guard let number = value as? NSNumber else { return "\(value)" }
        let double = number.doubleValue
        return double == double.rounded() && abs(double) < 1e15
            ? String(Int(double))
            : String(double)
    }

    var selectedOption: ParameterOption? {
        options.first { $0.label == value }
    }
}

// The load dropped unreadable parameters with compactMap and then reported on the SURVIVORS, so
// three failures out of a thousand gave a list of 997 and no sign that three were missing. A
// parameter browser is exactly where somebody goes to check whether a setting exists, and it was
// answering "not there" for both "the vehicle does not have it" and "the bridge could not read it".
//
// Worse when every read failed: the vehicle HAD named its parameters, so an empty result meant
// "none of the N it listed could be read" and the head said "This vehicle reported no parameters"
// -- the opposite of what happened, and the sentence that belongs to the other case. Both counts
// are in hand at that point, so telling them apart costs nothing.
enum ParameterLoad {
    static func status(named: Int, loaded: Int) -> String {
        guard named > 0 else { return "This vehicle reported no parameters." }
        guard loaded > 0 else {
            return "The vehicle listed \(named) parameters and none of them could be read."
        }
        guard loaded < named else { return "" }
        let missing = named - loaded
        return missing == 1
            ? "1 of \(named) parameters could not be read."
            : "\(missing) of \(named) parameters could not be read."
    }
}
