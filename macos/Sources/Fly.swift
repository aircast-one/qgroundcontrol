import Foundation

final class FlyStore: ObservableObject, Probeable {
    static let probeID = "fly"

    @Published private(set) var telemetry = FlyTelemetry()
    @Published private(set) var connected = false
    @Published private(set) var position: VehicleMarker?
    @Published private(set) var messages: [VehicleMessage] = []

    private var poll: Timer?

    func start() {
        refresh()
        poll?.invalidate()
        poll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stop() {
        poll?.invalidate()
        poll = nil
    }

    func refresh() {
        let vehicle = Bridge.group("vehicle")
        guard vehicle["kind"] as? String == "object" else {
            if connected { connected = false }
            if telemetry != FlyTelemetry() { telemetry = FlyTelemetry() }
            if position != nil { position = nil }
            if !messages.isEmpty { messages = [] }
            return
        }

        var reading = FlyTelemetry()
        reading.mode = (vehicle["flightMode"] as? String) ?? ""
        reading.armed = (vehicle["armed"] as? NSNumber)?.boolValue ?? false
        reading.flying = (vehicle["flying"] as? NSNumber)?.boolValue ?? false

        let facts = FlyStore.facts(vehicle)
        reading.altitude = facts["altitudeRelative"]
        reading.groundSpeed = facts["groundSpeed"]
        reading.climbRate = facts["climbRate"]
        reading.heading = facts["heading"]

        let gps = FlyStore.facts(Bridge.group("vehicle.gps"))
        reading.satellites = gps["count"].map { Int($0) }
        reading.gpsLock = gps["lock"].map { Int($0) }

        let battery = ((Bridge.group("vehicle.batteries")["elements"] as? [[String: Any]]) ?? []).first
        let batteryFacts = battery.map(FlyStore.facts) ?? [:]
        reading.batteryPercent = batteryFacts["percentRemaining"]
        reading.batteryVolts = batteryFacts["voltage"]

        let coordinate = vehicle["coordinate"] as? [String: Any]
        let placed = VehicleMarker(
            latitude: (coordinate?["latitude"] as? NSNumber)?.doubleValue,
            longitude: (coordinate?["longitude"] as? NSNumber)?.doubleValue,
            heading: reading.heading)

        if !connected { connected = true }
        if reading != telemetry { telemetry = reading }
        if placed != position { position = placed }

        let heard = VehicleMessage.parse((vehicle["formattedMessages"] as? String) ?? "")
        if heard != messages { messages = heard }
    }

    private static func facts(_ object: [String: Any]) -> [String: Double] {
        ((object["facts"] as? [[String: Any]]) ?? []).reduce(into: [String: Double]()) { values, fact in
            guard let name = fact["name"] as? String,
                  let value = (fact["value"] as? NSNumber)?.doubleValue, value.isFinite else { return }
            values[name] = value
        }
    }

    var latestMessages: [VehicleMessage] { Array(messages.prefix(FlyStore.messageLimit)) }

    static let messageLimit = 6

    func probeState() -> [String: Any] {
        ["connected": connected, "mode": telemetry.mode, "state": telemetry.stateText,
         "altitude": FlyTelemetry.metres(telemetry.altitude),
         "groundSpeed": FlyTelemetry.speed(telemetry.groundSpeed),
         "heading": FlyTelemetry.degrees(telemetry.heading),
         "battery": telemetry.batteryText, "gps": telemetry.gpsText,
         "placed": position != nil,
         "worstMessage": VehicleMessage.worst(latestMessages).rawValue,
         "messages": latestMessages.map { ["time": $0.time, "text": $0.text, "level": $0.level.rawValue] }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else { return ["ok": false, "error": "unknown action \(action)"] }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
