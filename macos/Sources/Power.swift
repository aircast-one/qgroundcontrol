import Foundation

final class PowerStore: ObservableObject, Probeable {
    static let probeID = "power"

    @Published private(set) var battery = BatteryReading.unavailable

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
        let batteries = (Bridge.group("vehicle.batteries")["elements"] as? [[String: Any]]) ?? []
        guard let first = batteries.first else {
            if battery != .unavailable { battery = .unavailable }
            return
        }

        let facts = (first["facts"] as? [[String: Any]]) ?? []
        func value(_ name: String) -> Double? {
            guard let raw = facts.first(where: { $0["name"] as? String == name })?["value"],
                  let number = raw as? NSNumber else { return nil }
            return number.doubleValue.isFinite ? number.doubleValue : nil
        }

        let reading = BatteryReading(voltage: value("voltage"),
                                     current: value("current"),
                                     percent: value("percentRemaining"))
        if reading != battery { battery = reading }
    }

    func probeState() -> [String: Any] {
        ["available": battery.available, "voltage": battery.voltageText,
         "current": battery.currentText, "percent": battery.percentText]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
