import Foundation

enum Units {
    static func display(_ units: String) -> String {
        units == "v" ? "V" : units
    }
}

struct DetailRow: Identifiable, Equatable {
    let label: String
    let value: String

    var id: String { label }
}

struct FactReading: Equatable {
    let name: String
    let value: String
    let units: String

    init(name: String, value: String, units: String) {
        self.name = name
        self.value = value
        self.units = units
    }

    init?(json: Any?) {
        guard let object = json as? [String: Any],
              let name = object["name"] as? String,
              let value = object["valueString"] as? String else { return nil }
        self.name = name
        self.value = value
        units = (object["units"] as? String) ?? ""
    }

    static func from(_ elements: [Any]) -> [FactReading] {
        elements.compactMap(FactReading.init(json:))
    }
}

enum FlyDetail {
    static let unreported = ["--.--", "--:--:--", "", "0.00 s"]

    static func rows(_ facts: [FactReading], _ wanted: [(String, String)]) -> [DetailRow] {
        wanted.compactMap { name, label in
            guard let fact = facts.first(where: { $0.name == name }) else { return nil }
            guard !unreported.contains(fact.value) else { return nil }
            let units = Units.display(fact.units)
            let value = units.isEmpty ? fact.value : "\(fact.value) \(units)"
            return DetailRow(label: label, value: value)
        }
    }

    static func battery(_ facts: [FactReading]) -> [DetailRow] {
        rows(facts, [("voltage", "Voltage"), ("current", "Current"),
                     ("instantPower", "Power"), ("mahConsumed", "Consumed"),
                     ("timeRemainingStr", "Time left"), ("temperature", "Temperature")])
    }

    static func gps(_ facts: [FactReading]) -> [DetailRow] {
        let position = ["lat", "lon"].compactMap { name in
            facts.first { $0.name == name }?.value
        }
        let placed = position.count == 2
            ? [DetailRow(label: "Position", value: position.joined(separator: ", "))]
            : []
        return placed + rows(facts, [("count", "Satellites"), ("hdop", "HDOP"),
                                     ("vdop", "VDOP"),
                                     ("courseOverGround", "Course over ground"),
                                     ("mgrs", "MGRS")])
    }

    // Vehicle::_remoteControlRSSIChanged uses 255 for invalid or unknown and stores an explicit
    // 0 once the filtered signal decays away, so the two are not the same answer: 255 means the
    // vehicle never told us, 0 means it told us the RC link is gone. Anything else outside
    // 0...100 is not a percentage QGC will accept either.
    static let rcRange = 0...100
    static let rcSilent = "No signal"

    // 255 is the sentinel and needs no clause of its own: it falls outside the percentage range
    // along with every other value QGC's own rcRSSI > 0 && <= 100 test rejects. Writing it out
    // separately read as protection and was dead - removing it failed nothing.
    static func rcSignal(_ value: Int) -> String? {
        guard rcRange.contains(value) else { return nil }
        return value == rcRange.lowerBound ? rcSilent : "\(value)%"
    }

    static func link(rcRSSI: Int?, localRSSI: Int?, remoteRSSI: Int?) -> [DetailRow] {
        let rc = rcRSSI.flatMap { value -> DetailRow? in
            rcSignal(value).map { DetailRow(label: "RC signal", value: $0) }
        }
        // Unlike the RC percentage, a telemetry dBm of 0 is the unset value: Vehicle initialises
        // both to 0 and QGC's own TelemetryRSSIIndicator reads telemetryLRSSI !== 0 as "there is
        // a telemetry radio at all". Dropping it is right here and the two cases do not match.
        let local = localRSSI.flatMap { value -> DetailRow? in
            guard value != 0 else { return nil }
            return DetailRow(label: "Telemetry here", value: "\(value) dBm")
        }
        let remote = remoteRSSI.flatMap { value -> DetailRow? in
            guard value != 0 else { return nil }
            return DetailRow(label: "Telemetry on the vehicle", value: "\(value) dBm")
        }
        return [rc, local, remote].compactMap { $0 }
    }
}
