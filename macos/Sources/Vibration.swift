import Foundation

final class VibrationStore: ObservableObject, Probeable {
    static let probeID = "vibration"

    @Published private(set) var reading = VibrationReading.unavailable

    private var timer: Timer?

    func start() {
        guard timer == nil else { return }
        refresh()
        let timer = Timer(timeInterval: 0.5, repeats: true) { [weak self] _ in self?.refresh() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    func refresh() {
        let group = Bridge.group("vehicle.vibration")
        let facts = (group["facts"] as? [[String: Any]]) ?? []
        guard !facts.isEmpty else {
            reading = .unavailable
            return
        }

        func value(_ name: String) -> Double? {
            guard let raw = facts.first(where: { $0["name"] as? String == name })?["value"] else { return nil }
            guard let number = raw as? NSNumber else { return nil }
            let double = number.doubleValue
            // The vehicle publishes NaN until it reports vibration at all.
            return double.isFinite ? double : nil
        }

        guard let x = value("xAxis"), let y = value("yAxis"), let z = value("zAxis") else {
            reading = .unavailable
            return
        }

        reading = VibrationReading(
            x: x, y: y, z: z,
            clipCounts: (1...3).map { Int(value("clipCount\($0)") ?? 0) },
            available: true)
    }

    func probeState() -> [String: Any] {
        ["available": reading.available,
         "x": reading.x, "y": reading.y, "z": reading.z,
         "clipCounts": reading.clipCounts,
         "worst": String(describing: reading.worst)]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
