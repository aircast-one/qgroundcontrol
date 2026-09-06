import Foundation

struct Fact: Identifiable {
    enum Kind {
        case toggle
        case choice(labels: [String], values: [Int])
        case text
        case number(decimalPlaces: Int, minimum: Double?, maximum: Double?)
    }

    let path: String
    let name: String
    let title: String
    let units: String
    let readOnly: Bool
    let kind: Kind
    let value: Any

    var id: String { path }

    init?(json: [String: Any], groupPath: String) {
        guard let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        path = "\(groupPath).\(name)"
        let described = (json["shortDescription"] as? String) ?? ""
        title = described.isEmpty ? Fact.humanise(name) : described
        units = (json["units"] as? String) ?? ""
        readOnly = (json["readOnly"] as? NSNumber)?.boolValue ?? false
        value = json["value"] ?? NSNull()

        let labels = (json["enumStrings"] as? [String]) ?? []
        let rawValues = (json["enumValues"] as? [Any]) ?? []
        let values = rawValues.map { ($0 as? NSNumber)?.intValue ?? 0 }

        if (json["typeIsBool"] as? NSNumber)?.boolValue == true {
            kind = .toggle
        } else if !labels.isEmpty {
            // enumValues can be absent even when enumStrings is not; index by position then.
            kind = .choice(labels: labels, values: values.count == labels.count ? values : Array(0..<labels.count))
        } else if (json["typeIsString"] as? NSNumber)?.boolValue == true {
            kind = .text
        } else {
            kind = .number(
                decimalPlaces: (json["decimalPlaces"] as? NSNumber)?.intValue ?? 0,
                minimum: Fact.bound(json["min"]),
                maximum: Fact.bound(json["max"]))
        }
    }

    var boolValue: Bool { (value as? NSNumber)?.boolValue ?? false }
    var intValue: Int { (value as? NSNumber)?.intValue ?? 0 }
    var stringValue: String {
        if let text = value as? String { return text }
        guard let number = value as? NSNumber else { return "" }
        if case let .number(places, _, _) = kind, places > 0 {
            return String(format: "%.\(places)f", number.doubleValue)
        }
        return number.doubleValue == number.doubleValue.rounded()
            ? String(number.intValue)
            : String(number.doubleValue)
    }

    // QGC leaves min/max at the type's extremes when a fact is unbounded; showing
    // "0 – 4294967295" as a hint is worse than showing nothing.
    // Above this a bound is the type's extreme rather than a real limit; UINT32_MAX as
    // a "maximum" hint is noise, not guidance.
    private static let sentinelBound = 1e9

    private static func bound(_ raw: Any?) -> Double? {
        guard let number = raw as? NSNumber else { return nil }
        let value = number.doubleValue
        guard value.isFinite, abs(value) < sentinelBound else { return nil }
        return value
    }

    // Split on lower->upper only. Splitting every capital turns "RemoteID" into
    // "Remote I D" and "ADSBVehicleManager" into "Adsb Vehicle Manager", which
    // destroys the scent an operator scans the sidebar for.
    static func humanise(_ identifier: String) -> String {
        let characters = Array(identifier)
        var words: [String] = []
        var current = ""

        for (index, character) in characters.enumerated() {
            let previous = index > 0 ? characters[index - 1] : nil
            let next = index + 1 < characters.count ? characters[index + 1] : nil
            let startsWord: Bool
            if let previous {
                if character.isUppercase, previous.isLowercase {
                    // not previous.isNumber: "3D" and "H264" are one word, not two
                    startsWord = true
                } else if character.isUppercase, previous.isUppercase, let next, next.isLowercase {
                    // trailing capital of a run begins the next word: "ADSBVehicle" -> ADSB | Vehicle
                    startsWord = true
                } else if character.isNumber, previous.isLetter, !previous.isUppercase {
                    startsWord = true
                } else {
                    startsWord = false
                }
            } else {
                startsWord = false
            }

            if startsWord, !current.isEmpty {
                words.append(current)
                current = ""
            }
            current.append(character)
        }
        if !current.isEmpty { words.append(current) }

        return words
            .map { word -> String in
                let upper = word.uppercased()
                if acronyms.contains(upper) { return upper }
                return word.first!.isUppercase ? word : word.prefix(1).uppercased() + word.dropFirst()
            }
            .joined(separator: " ")
    }

    // Identifiers that start lowercase lose their acronym casing entirely
    // ("rtkSettings" -> "Rtk"), which no casing rule can recover.
    private static let acronyms: Set<String> = [
        "ADSB", "AGL", "AMSL", "APM", "ESC", "GCS", "GPS", "ID", "IMU", "NMEA",
        "PX4", "RC", "RTK", "RTSP", "TCP", "UDP", "UVC", "VTOL", "UTM", "SBS", "3D",
    ]
}

struct SettingsGroup: Identifiable {
    let path: String
    let title: String
    var id: String { path }

    init(child: String) {
        path = "settings.\(child)"
        var name = child
        if name.hasSuffix("Settings") { name.removeLast("Settings".count) }
        title = Fact.humanise(name)
    }
}
