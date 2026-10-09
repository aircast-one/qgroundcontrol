import Foundation

let SEVERITY_SECONDARY = -1

struct DetailRow: Equatable, Hashable {
    let label: String
    let value: String
    var severity: Int = 0
}

struct BatteryHeadline: Equatable {
    let text: String
    let detail: String
    let severity: Int
    var level: BatteryLevel = .Normal
    var margin: String = ""
    var index: Int = 0
}

func batteryHeadline(_ view: JSON?) -> BatteryHeadline? {
    guard let view, view["available"].bool, let it = view["headline"].objectOrNil else { return nil }
    return BatteryHeadline(
        text: it["text"].string,
        detail: it["detail"].string,
        severity: it["severity"].int(0),
        level: batteryLevelOf(it["level"].string),
        margin: it["margin"].string,
        index: it["index"].int(0)
    )
}

private let LIMITING_PACK = "lowest"

func batteryDetail(_ view: JSON?) -> [DetailRow] {
    guard let view, view["available"].bool, let packs = view["packs"].arrayOrNil else { return [] }
    let rowsOf = { (index: Int) -> [JSON] in
        packs.indices.contains(index) ? packs[index]["rows"].objects : []
    }
    guard packs.count >= 2 else {
        return rowsOf(0).map { DetailRow(label: $0["label"].string, value: $0["value"].string, severity: $0["severity"].int(SEVERITY_SECONDARY)) }
    }
    let limiting = batteryHeadline(view)?.index
    return packs.indices.map { index in
        let rows = rowsOf(index)
        return DetailRow(
            label: ["Battery \(index + 1)", index == limiting ? LIMITING_PACK : nil].compactMap { $0 }.joined(separator: " \u{00b7} "),
            value: rows.map { $0["value"].string }.joined(separator: " \u{00b7} "),
            severity: max(rows.map { $0["severity"].int(SEVERITY_SECONDARY) }.max() ?? 0, 0)
        )
    }
}

func silenceText(_ seconds: Int64) -> String { "No data from the aircraft for \(silenceDuration(seconds))" }

let SIGNAL_LOST = "Signal lost"

func failsafeCountdown(_ seconds: Int64?, _ failsafe: LossFailsafe?) -> String? {
    guard let failsafe else { return nil }
    let left = failsafe.after.map { Int64(($0 - Double(seconds ?? 0)).rounded(.up)) }.flatMap { $0 > 0 ? $0 : nil }
    return left.map { "\(failsafe.action) in \(silenceDuration($0))" } ?? failsafe.action
}

func signalLostTitle(_ seconds: Int64?, failsafe: LossFailsafe? = nil) -> String {
    [SIGNAL_LOST, seconds.map(silenceDuration), failsafeCountdown(seconds, failsafe)].compactMap { $0 }.joined(separator: " \u{00b7} ")
}

private let CRITICAL_CHARGE_STATES = 3...6

func batteryReturnOffered(_ view: JSON?) -> Bool {
    guard let view, view["available"].bool, let packs = view["packs"].arrayOrNil else { return false }
    return packs.contains { $0.object != nil && CRITICAL_CHARGE_STATES.contains($0["chargeState"].int(0)) }
}

func totalDraw(_ view: JSON?) -> String? {
    guard let view, view["available"].bool, let packs = view["packs"].arrayOrNil else { return nil }
    let number = { (pack: JSON, name: String) -> Double? in
        pack["facts"].array.first { $0.object != nil && $0["name"].string == name }?["value"].double.flatMap { $0.isNaN ? nil : $0 }
    }
    let all = packs.filter { $0.object != nil }
    let watts = all.map { number($0, "instantPower") }
    let amps = all.map { number($0, "current") }
    if all.isEmpty { return nil }
    if watts.allSatisfy({ $0 != nil }) { return "\(Int64((watts.compactMap { $0 }.reduce(0, +) + 0.5).rounded(.down)))W" }
    if amps.allSatisfy({ $0 != nil }) { return String(format: "%.1fA", amps.compactMap { $0 }.reduce(0, +)) }
    return nil
}

func factLabel(_ name: String) -> String {
    switch name {
    case "voltage": "Voltage"
    case "current": "Current"
    case "instantPower": "Power"
    case "mahConsumed": "Consumed"
    case "timeRemainingStr": "Time remaining"
    case "temperature": "Temperature"
    case "percentRemaining": "Remaining"
    default: name.capitalizedFirst
    }
}

struct GpsStatus {
    let satellites: Int?
    let lock: Double
    let lockText: String
    let rows: [DetailRow]
}

func gpsStatus(_ view: JSON?) -> GpsStatus? {
    guard let view, view["available"].bool else { return nil }
    return GpsStatus(
        satellites: view["satellites"].isNull ? nil : view["satellites"].int(0),
        lock: view["lock"].isNull ? .nan : view["lock"].double(.nan),
        lockText: view["lockText"].string,
        rows: view["rows"].array
            .filter { $0.object != nil }
            .map { DetailRow(label: $0["label"].string, value: $0["value"].string) }
            .filter { !$0.label.isBlank && !$0.value.isBlank }
    )
}

func gpsDetail(_ gps: GpsStatus?) -> [DetailRow] {
    [gps?.lockText].compactMap { $0 }.filter { !$0.isBlank }.map { DetailRow(label: "GPS Lock", value: $0) } + (gps?.rows ?? [])
}

func notYetComputed(_ shown: String) -> Bool {
    shown.isBlank || shown.allSatisfy { $0 == "-" || $0 == ":" || $0 == "." || $0 == " " }
}

func usableDop(_ shown: String) -> Bool {
    !notYetComputed(shown) && (Double(shown).map { $0 > 0 && $0 < 100 } ?? false)
}

func linkDetail(_ links: VehicleLinks?, _ names: [String], _ primary: String?) -> [DetailRow] {
    guard let links, links.available else { return [] }
    return names.enumerated().map { index, name in
        let lost = links.links.indices.contains(index) && links.links[index].commLost == true
        let notes = [name == primary ? "carrying" : nil, lost ? "no contact" : nil].compactMap { $0 }
        return DetailRow(label: name, value: (notes.isEmpty ? ["standing by"] : notes).joined(separator: " · "))
    }
}
