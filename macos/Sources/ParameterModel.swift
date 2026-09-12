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

    var id: String { "\(componentId)/\(name)" }
    var path: String { "vehicle.parameterManager.getParameter(\(componentId),\(name))" }

    var group: String {
        guard let underscore = name.firstIndex(of: "_") else { return name }
        return String(name[name.startIndex..<underscore])
    }

    init(name: String, componentId: Int, json: [String: Any]) {
        self.name = name
        self.componentId = componentId
        units = (json["units"] as? String) ?? ""
        description = (json["shortDescription"] as? String) ?? ""

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
