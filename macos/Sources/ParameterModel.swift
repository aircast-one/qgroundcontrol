import Foundation

struct ParameterOption: Identifiable, Equatable {
    let label: String
    let raw: String

    var id: String { raw }
}

struct Parameter: Identifiable {
    // Fact::enumIndex appends tr("Unknown: %1").arg(rawValue()) when the current value is not
    // among the declared ones, and Fact::unknownEnumLabel now returns that SAME expression, so
    // the two are equal by construction in every locale. Filtering on the English prefix offered
    // the bogus entry in the picker outside English and showed "Unbekannt: 9" where English
    // showed "9"; no head-side test could tell the synthetic entry apart, because addEnumInfo
    // mutates the metadata permanently and it becomes structurally identical to a real last entry.
    static func synthetic(_ label: String, unknownEnumLabel: String) -> Bool {
        !unknownEnumLabel.isEmpty && label == unknownEnumLabel
    }

    let name: String
    let componentId: Int
    let value: String
    let units: String
    let description: String
    let options: [ParameterOption]
    let range: FactRange
    // Served in kFactProperties all along and read by nothing. Without it this model cannot tell
    // a string-valued parameter from a numeric one, which is why 37226d051 could tighten
    // SettingsControl.refusal -- it guards on kind first -- and had to leave FactRange alone.
    let isString: Bool

    var id: String { "\(componentId)/\(name)" }
    var path: String { "vehicle.parameterManager.getParameter(\(componentId),\(name))" }

    var group: String {
        guard let underscore = name.firstIndex(of: "_") else { return name }
        return String(name[name.startIndex..<underscore])
    }

    init(name: String, componentId: Int, json: [String: Any]) {
        self.name = name
        self.componentId = componentId
        isString = (json["typeIsString"] as? NSNumber)?.boolValue ?? false
        units = (json["units"] as? String) ?? ""
        description = (json["shortDescription"] as? String) ?? ""
        range = FactRange(json, title: name)

        let enumIndex = (json["enumIndex"] as? NSNumber)?.intValue ?? -1
        let enums = (json["enumStrings"] as? [String]) ?? []
        let raws = (json["enumValues"] as? [Any]) ?? []
        let unknownLabel = (json["unknownEnumLabel"] as? String) ?? ""
        options = enums.count == raws.count
            ? zip(enums, raws)
                .filter { !Parameter.synthetic($0.0, unknownEnumLabel: unknownLabel) }
                .map { ParameterOption(label: $0.0, raw: Parameter.rawText($0.1)) }
            : []
        let plain = (json["valueString"] as? String) ?? ""
        if enumIndex >= 0, enumIndex < enums.count,
           !Parameter.synthetic(enums[enumIndex], unknownEnumLabel: unknownLabel) {
            value = enums[enumIndex]
        } else {
            value = plain
        }
    }
}

extension Parameter {
    // The numeric rules apply to numeric parameters only. A string parameter legitimately holds
    // a comma, a letter or anything else, and refusing it would break a write that is correct --
    // worse than the hole it closes.
    func refusal(_ entry: String) -> String? {
        guard !isString else { return nil }
        if let refused = Measure.numberRefusal(entry) { return refused }
        return range.refusal(entry)
    }

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
