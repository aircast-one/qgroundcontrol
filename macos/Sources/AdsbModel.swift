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
    }

    // Being connected to a feed and hearing nothing from it are different states, and an empty
    // sky is a third. Only the last one is good news, so they must not share a rendering.
    var quiet: Bool { connected && !receiving }

    var alertsKnown: Bool { alertUnknown == 0 }
}
