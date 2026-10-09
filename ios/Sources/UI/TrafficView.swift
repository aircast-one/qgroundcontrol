import Foundation

let TRAFFIC_VIEW = "view.adsbTraffic"

enum TrafficLevel: Int, Comparable {
    case Good, Caution, Warning, Critical

    static func < (lhs: TrafficLevel, rhs: TrafficLevel) -> Bool { lhs.rawValue < rhs.rawValue }
}

struct TrafficContact: Equatable, Identifiable {
    let icaoAddress: Int
    let callsign: String
    let distance: Double?
    let bearingDegrees: Double?
    let altitude: Double?
    let relativeAltitude: Double?
    let emergency: String
    let alert: Bool?
    let stale: Bool
    var altitudeType: String = ""
    var simulated: Bool = false

    var name: String { callsign.ifBlank(String(format: "%06X", icaoAddress)) }
    var located: Bool { distance != nil && bearingDegrees != nil }
    var id: Int { icaoAddress }
}

struct TrafficUnits: Equatable {
    let distance: String
    let altitude: String
    let heading: String
}

struct TrafficReading: Equatable {
    let enabled: Bool
    let available: Bool
    let connected: Bool
    let receiving: Bool
    let ownPositionKnown: Bool
    let alerting: Bool?
    let alertUnknown: Int
    let emergency: String
    let errorToken: String
    let units: TrafficUnits
    let contacts: [TrafficContact]
}

private func number(_ json: JSON, _ name: String) -> Double? { json[name].double.flatMap { $0.isFinite ? $0 : nil } }

func trafficReading(_ view: JSON?) -> TrafficReading? {
    guard let view, view["class"].string == "AdsbTraffic" else { return nil }
    let units = view["units"]
    return TrafficReading(
        enabled: view["enabled"].bool,
        available: view["available"].bool,
        connected: view["connected"].bool,
        receiving: view["receiving"].bool,
        ownPositionKnown: view["ownPositionKnown"].bool,
        alerting: view["alerting"].boolOrNil,
        alertUnknown: view["alertUnknown"].int(0),
        emergency: view["emergency"].string,
        errorToken: view["error"]["token"].string,
        units: TrafficUnits(distance: units["distance"].string, altitude: units["altitude"].string, heading: units["heading"].string),
        contacts: view["contacts"].objects.map { contact in
            TrafficContact(
                icaoAddress: contact["icaoAddress"].int(0),
                callsign: contact["callsign"].string.trimmed,
                distance: number(contact, "distance"),
                bearingDegrees: number(contact, "bearingDegrees"),
                altitude: number(contact, "altitude"),
                relativeAltitude: number(contact, "relativeAltitude"),
                emergency: contact["emergency"].string,
                alert: contact["alert"].boolOrNil,
                stale: contact["stale"].bool,
                altitudeType: contact["altitudeType"].string,
                simulated: contact["simulated"].bool
            )
        }
    )
}

func trafficShown(_ reading: TrafficReading) -> Bool { reading.enabled || !reading.contacts.isEmpty }

func trafficLevel(_ reading: TrafficReading) -> TrafficLevel {
    if !reading.emergency.isBlank { return .Critical }
    if reading.alerting == true { return .Warning }
    if !reading.errorToken.isBlank { return .Caution }
    if !reading.receiving { return .Caution }
    if reading.alertUnknown > 0 && reading.connected { return .Caution }
    return .Good
}

func trafficReal(_ reading: TrafficReading) -> Int { reading.contacts.filter { !$0.simulated }.count }

func trafficSynthetic(_ reading: TrafficReading) -> Int { reading.contacts.filter(\.simulated).count }

private func feedFault(_ reading: TrafficReading) -> String? {
    if reading.errorToken == "connectFailed" { return "Traffic server unreachable" }
    if reading.errorToken == "linkLost" { return "Traffic feed dropped" }
    if !reading.errorToken.isBlank { return "Traffic feed failed" }
    if !reading.available { return "No traffic receiver" }
    if !reading.receiving { return "No traffic feed" }
    return nil
}

func trafficSummary(_ reading: TrafficReading) -> String {
    let real = trafficReal(reading)
    let synthetic = trafficSynthetic(reading)
    if real > 0 && synthetic > 0 { return "Traffic: \(real) aircraft \u{00b7} \(synthetic) simulated" }
    if real > 0 { return "Traffic: \(real) aircraft" }
    if synthetic > 0 { return "Traffic: \(synthetic) simulated" }
    return feedFault(reading) ?? "Traffic clear"
}

func trafficContactUrgent(_ contact: TrafficContact) -> Bool { contact.alert == true || !contact.emergency.isBlank }

func trafficCaption(_ reading: TrafficReading) -> String {
    reading.ownPositionKnown ? "Range, bearing and height are relative to the vehicle" : "No vehicle position, so nothing can be ranged"
}

func trafficEmergencyText(_ token: String) -> String {
    switch token {
    case "hijack": "squawking hijack"
    case "radioFailure": "squawking radio failure"
    case "general": "squawking emergency"
    default: ""
    }
}

private func reading(_ value: Double?, _ unit: String, fine: Bool = false) -> String {
    guard let number = value else { return "" }
    let printed = fine && abs(number) < 10 ? String(format: "%.1f", number) : String(format: "%.0f", number)
    return unit.isBlank ? printed : "\(printed) \(unit)"
}

func trafficDatumText(_ altitudeType: String) -> String {
    switch altitudeType {
    case "pressureQnh": "by pressure"
    case "geometric": "by GPS"
    default: ""
    }
}

func trafficHeightText(_ contact: TrafficContact, _ units: TrafficUnits) -> String {
    guard let relative = contact.relativeAltitude else {
        return [reading(contact.altitude, units.altitude), trafficDatumText(contact.altitudeType)].filter { !$0.isBlank }.joined(separator: " ")
    }
    let printed = reading(abs(relative), units.altitude)
    if abs(relative) < 0.5 { return "my level" }
    return relative > 0 ? "\(printed) above" : "\(printed) below"
}

func trafficContactText(_ contact: TrafficContact, _ units: TrafficUnits) -> String {
    let position = contact.located
        ? [reading(contact.distance, units.distance, fine: true), reading(contact.bearingDegrees, units.heading), trafficHeightText(contact, units)]
        : ["bearing unknown", trafficHeightText(contact, units)]
    let trailing = [
        trafficEmergencyText(contact.emergency),
        contact.alert == true ? "alerting" : "",
        contact.stale ? "stale" : "",
    ]
    let synthetic = contact.simulated ? "simulated" : ""
    return ([synthetic] + position + trailing).filter { !$0.isBlank }.joined(separator: "  ")
}

struct TrafficAlert: Equatable {
    let level: TrafficLevel
    let title: String
    let detail: String
}

private let COMPASS_POINTS = ["N", "NE", "E", "SE", "S", "SW", "W", "NW"]

func compassPoint(_ bearingDegrees: Double) -> String {
    let step = Int((bearingDegrees / 45 + 0.5).rounded(.down))
    return COMPASS_POINTS[((step % COMPASS_POINTS.count) + COMPASS_POINTS.count) % COMPASS_POINTS.count]
}

func trafficThreat(_ reading: TrafficReading) -> TrafficContact? {
    reading.contacts.filter(trafficContactUrgent).min { ($0.distance ?? .greatestFiniteMagnitude) < ($1.distance ?? .greatestFiniteMagnitude) }
}

func trafficThreatText(_ contact: TrafficContact, _ units: TrafficUnits) -> String {
    [
        contact.name,
        [reading(contact.distance, units.distance, fine: true), contact.bearingDegrees.map(compassPoint) ?? ""].filter { !$0.isBlank }.joined(separator: " "),
        trafficHeightText(contact, units),
    ].filter { !$0.isBlank }.joined(separator: " \u{00b7} ")
}

func trafficAlert(_ reading: TrafficReading) -> TrafficAlert? {
    let level = trafficLevel(reading)
    guard trafficShown(reading), level != .Good else { return nil }
    let threat = level >= .Warning ? trafficThreat(reading) : nil
    let title = switch level {
    case .Critical: "Aircraft \(trafficEmergencyText(reading.emergency).ifBlank("in emergency"))"
    case .Warning: "Aircraft nearby"
    default: feedFault(reading) ?? "Collision alerts unknown for \(reading.alertUnknown) aircraft"
    }
    return TrafficAlert(level: level, title: title, detail: threat.map { trafficThreatText($0, reading.units) } ?? "")
}
