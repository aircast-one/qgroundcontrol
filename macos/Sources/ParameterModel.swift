import Foundation

struct Parameter: Identifiable {
    static let unknownEnumPrefix = "Unknown: "

    let name: String
    let componentId: Int
    let value: String
    let units: String
    let description: String

    var id: String { "\(componentId)/\(name)" }
    // getParameter is reachable as a path segment, so a parameter needs no root of
    // its own: the bridge resolves the method call and returns the Fact behind it.
    var path: String { "vehicle.parameterManager.getParameter(\(componentId),\(name))" }

    // A parameter's group is the prefix before the first underscore: ATC_ANG_PIT_P
    // belongs to ATC. It is how ArduPilot and PX4 documentation is organised, and how
    // an operator narrows 1390 parameters to the dozen they care about.
    var group: String {
        guard let underscore = name.firstIndex(of: "_") else { return name }
        return String(name[name.startIndex..<underscore])
    }

    init(name: String, componentId: Int, json: [String: Any]) {
        self.name = name
        self.componentId = componentId
        units = (json["units"] as? String) ?? ""
        description = (json["shortDescription"] as? String) ?? ""

        // When a value falls outside its enum, QGC appends a synthetic "Unknown: N"
        // entry and points enumIndex at it. Faithfully showing that turns a perfectly
        // ordinary 0-second time constant into "Unknown: 0"; the number is better.
        let enumIndex = (json["enumIndex"] as? NSNumber)?.intValue ?? -1
        let enums = (json["enumStrings"] as? [String]) ?? []
        let plain = (json["valueString"] as? String) ?? ""
        if enumIndex >= 0, enumIndex < enums.count, !enums[enumIndex].hasPrefix(Parameter.unknownEnumPrefix) {
            value = enums[enumIndex]
        } else {
            value = plain
        }
    }
}
