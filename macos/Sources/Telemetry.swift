import Foundation
import SwiftUI

struct Gauge: Identifiable {
    let id = UUID()
    let label: String
    let path: String
    var reading = Bridge.Reading.absent
}

final class Telemetry: ObservableObject {
    static let budgetMillis = 16.0

    @Published private(set) var gauges: [Gauge]
    @Published private(set) var connected = false
    @Published private(set) var armed = false
    @Published private(set) var vehicleType = "—"
    @Published private(set) var flightMode = "—"
    @Published private(set) var lastTickMillis = 0.0
    @Published private(set) var worstTickMillis = 0.0
    @Published private(set) var ticks = 0
    @Published private(set) var overBudget = 0
    @Published private(set) var events: [String] = []

    private var timer: Timer?

    init() {
        gauges = [
            Gauge(label: "Roll", path: "vehicle.vehicle.roll"),
            Gauge(label: "Pitch", path: "vehicle.vehicle.pitch"),
            Gauge(label: "Heading", path: "vehicle.vehicle.heading"),
            Gauge(label: "Altitude (rel)", path: "vehicle.vehicle.altitudeRelative"),
            Gauge(label: "Ground speed", path: "vehicle.vehicle.groundSpeed"),
            Gauge(label: "Climb rate", path: "vehicle.vehicle.climbRate"),
        ]
    }

    func start() {
        Bridge.onEvent { [weak self] path, json in
            self?.record(event: path, json: json)
        }
        Bridge.watch(["vehicle.flightMode", "vehicle.armed", "vehicles.activeVehicleAvailable"])

        let timer = Timer(timeInterval: 0.1, repeats: true) { [weak self] _ in self?.tick() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
        tick()
    }

    func stop() {
        timer?.invalidate()
        timer = nil
        Bridge.watch([])
    }

    private func tick() {
        let started = DispatchTime.now().uptimeNanoseconds

        let available = Bridge.bool("vehicles.activeVehicleAvailable")
        let type = Bridge.read("vehicle.vehicleTypeString")
        let mode = Bridge.read("vehicle.flightMode")
        let isArmed = Bridge.bool("vehicle.armed")
        let readings = gauges.map { Bridge.read($0.path) }

        let elapsed = Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000

        connected = available
        vehicleType = type.text
        flightMode = mode.text
        armed = isArmed
        gauges = zip(gauges, readings).map { gauge, reading in
            var updated = gauge
            updated.reading = reading
            return updated
        }

        lastTickMillis = elapsed
        worstTickMillis = max(worstTickMillis, elapsed)
        ticks += 1
        overBudget += elapsed > Self.budgetMillis ? 1 : 0
    }

    private func record(event path: String, json: [String: Any]) {
        let stamp = Self.stampFormatter.string(from: Date())
        let value = json["value"].map { "\($0)" } ?? json["kind"] as? String ?? "?"
        events = ([("\(stamp)  \(path) → \(value)")] + events).prefix(8).map { $0 }
    }

    func resetMetrics() {
        worstTickMillis = 0
        ticks = 0
        overBudget = 0
    }

    private static let stampFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.dateFormat = "HH:mm:ss.SSS"
        return formatter
    }()
}
