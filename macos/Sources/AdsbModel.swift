import Foundation

struct AdsbContact: Equatable {
    let icaoAddress: Int
    let callsign: String
    let point: GeoPoint?
    let altitudeMetres: Double?
    let headingDegrees: Double?
    let velocityMetresPerSecond: Double?
    let distanceMetres: Double?
    let bearingDegrees: Double?
    let relativeAltitudeMetres: Double?
    let distance: Double?
    let altitude: Double?
    let velocity: Double?
    let relativeAltitude: Double?
    let squawk: Int?
    let emergency: String
    let alert: Bool?
    let stale: Bool
    let simulated: Bool

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let icao = (json["icaoAddress"] as? NSNumber)?.intValue else { return nil }
        func number(_ name: String) -> Double? {
            guard let value = (json[name] as? NSNumber)?.doubleValue, value.isFinite else {
                return nil
            }
            return value
        }
        icaoAddress = icao
        callsign = ((json["callsign"] as? String) ?? "").trimmingCharacters(in: .whitespaces)
        point = GeoPoint(json: json)
        altitudeMetres = number("altitudeMetres")
        headingDegrees = number("headingDegrees")
        velocityMetresPerSecond = number("velocityMetresPerSecond")
        distanceMetres = number("distanceMetres")
        bearingDegrees = number("bearingDegrees")
        relativeAltitudeMetres = number("relativeAltitudeMetres")
        distance = number("distance")
        altitude = number("altitude")
        velocity = number("velocity")
        relativeAltitude = number("relativeAltitude")
        squawk = (json["squawk"] as? NSNumber)?.intValue
        emergency = (json["emergency"] as? String) ?? ""
        alert = (json["alert"] as? NSNumber)?.boolValue
        stale = (json["stale"] as? NSNumber)?.boolValue ?? false
        simulated = (json["simulated"] as? NSNumber)?.boolValue ?? false
    }

    // The core withholds distance and bearing unless it knows where the operator is, so an
    // aircraft the receiver can hear but cannot place relative to us is a contact without a
    // range. Rendering that as 0 would put it on top of the operator.
    var located: Bool { distanceMetres != nil && bearingDegrees != nil }

    var name: String { callsign.isEmpty ? String(format: "%06X", icaoAddress) : callsign }
}

struct AdsbUnits: Equatable {
    let distance: String
    let altitude: String
    let velocity: String
    let heading: String

    init(_ json: Any?) {
        let json = (json as? [String: Any]) ?? [:]
        distance = (json["distance"] as? String) ?? ""
        altitude = (json["altitude"] as? String) ?? ""
        velocity = (json["velocity"] as? String) ?? ""
        heading = (json["heading"] as? String) ?? ""
    }
}

struct AdsbTraffic: Equatable {
    let enabled: Bool
    let available: Bool
    let connected: Bool
    let receiving: Bool
    let ownPositionKnown: Bool
    let contacts: [AdsbContact]
    let count: Int
    let alerting: Int
    let alertUnknown: Int
    let emergency: String
    let errorToken: String
    let errorDetail: String
    let units: AdsbUnits

    static let none = AdsbTraffic()

    private init() {
        enabled = false
        available = false
        connected = false
        receiving = false
        ownPositionKnown = false
        contacts = []
        count = 0
        alerting = 0
        alertUnknown = 0
        emergency = ""
        errorToken = ""
        errorDetail = ""
        units = AdsbUnits(nil)
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], json["kind"] as? String == "object" else {
            return nil
        }
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        enabled = flag("enabled")
        available = flag("available")
        connected = flag("connected")
        receiving = flag("receiving")
        ownPositionKnown = flag("ownPositionKnown")
        contacts = ((json["contacts"] as? [Any]) ?? []).compactMap(AdsbContact.init)
        count = (json["count"] as? NSNumber)?.intValue ?? 0
        alerting = (json["alerting"] as? NSNumber)?.intValue ?? 0
        alertUnknown = (json["alertUnknown"] as? NSNumber)?.intValue ?? 0
        emergency = (json["emergency"] as? String) ?? ""
        let error = json["error"] as? [String: Any]
        errorToken = (error?["token"] as? String) ?? ""
        errorDetail = (error?["detail"] as? String) ?? ""
        units = AdsbUnits(json["units"])
    }

    // Being connected to a feed and hearing nothing from it are different states, and an empty
    // sky is a third. Only the last one is good news, so they must not share a rendering.
    var quiet: Bool { connected && !receiving }

    var alertsKnown: Bool { alertUnknown == 0 }

    // The unit name comes from the block ONCE, never from the contact. A contact list is fifty
    // rows of one quantity, so a per-row unit would be fifty copies of a string the view states
    // in a single place -- which is why the core serves a converted number here and a *Text on
    // a singleton like the vehicle distance.
    func distanceText(_ contact: AdsbContact) -> String {
        Self.show(contact.distance, units.distance)
    }

    func altitudeText(_ contact: AdsbContact) -> String {
        Self.show(contact.altitude, units.altitude)
    }

    func velocityText(_ contact: AdsbContact) -> String {
        Self.show(contact.velocity, units.velocity)
    }

    func bearingText(_ contact: AdsbContact) -> String {
        Self.show(contact.bearingDegrees, units.heading)
    }

    static func show(_ value: Double?, _ unit: String) -> String {
        guard let value, value.isFinite else { return "" }
        let printed = Measure.settled(String(format: "%.0f", value.rounded()))
        return unit.isEmpty ? printed : printed + " " + unit
    }

    // A CONFIGURED RECEIVER THAT IS SWITCHED OFF IS NOT AN ABSENT ONE. The core reports
    // available from whether a source is set and enabled from whether the feature is on, and
    // collapsing them tells an operator who disabled traffic that they never had a receiver.
    var summary: String {
        guard available else { return "No receiver" }
        guard enabled else { return "Traffic off" }
        guard !quiet else { return "Silent" }
        guard !contacts.isEmpty else { return "Clear" }
        return contacts.count == 1 ? "1 aircraft" : "\(contacts.count) aircraft"
    }

    // An aircraft squawking an emergency outranks a merely close one, and an alert nobody
    // reported is not a calm sky -- so an unknown alert is drawn as caution rather than good.
    var level: FlyTelemetry.Level {
        guard available, enabled else { return .unknown }
        if !emergency.isEmpty { return .critical }
        if alerting > 0 { return .warning }
        if quiet || alertUnknown > 0 { return .caution }
        return .good
    }

    // The panel must not go blank in three different situations that mean different things, so
    // each gets a sentence rather than an empty list: no receiver configured, a receiver whose
    // feed has gone silent, and clear air. Only the last is good news.
    func rows() -> [DetailRow] {
        guard available else {
            return [DetailRow(label: "Traffic", value: "No receiver configured")]
        }
        guard enabled else {
            return [DetailRow(label: "Traffic", value: "Switched off")]
        }
        guard !quiet else {
            return [DetailRow(label: "Traffic", value: "Receiver silent")]
        }
        guard !contacts.isEmpty else {
            return [DetailRow(label: "Traffic", value: "No aircraft nearby")]
        }
        return contacts.map { contact in
            DetailRow(label: contact.name, value: describe(contact))
        }
    }

    // An aircraft the receiver hears but cannot place says so, rather than showing a blank
    // column that reads as a decoding fault.
    func describe(_ contact: AdsbContact) -> String {
        guard contact.located else { return "bearing unknown" }
        return [distanceText(contact), bearingText(contact), altitudeText(contact),
                velocityText(contact)]
            .filter { !$0.isEmpty }
            .joined(separator: "  ")
    }
}
