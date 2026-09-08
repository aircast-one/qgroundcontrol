import Foundation

final class PowerStore: ObservableObject, Probeable {
    static let probeID = "power"

    @Published private(set) var battery = BatteryReading.unavailable
    @Published private(set) var level = FlyTelemetry.Level.unknown

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
        let view = Bridge.group("view.battery")
        guard let first = (view["packs"] as? [[String: Any]])?.first else {
            if battery != .unavailable { battery = .unavailable }
            if level != .unknown { level = .unknown }
            return
        }

        func value(_ name: String) -> Double? {
            guard let number = first[name] as? NSNumber, number.doubleValue.isFinite else {
                return nil
            }
            return number.doubleValue
        }

        let reading = BatteryReading(voltage: value("voltage"),
                                     current: value("current"),
                                     percent: value("percent"))
        if reading != battery { battery = reading }
        let read = FlyTelemetry.Level(view["level"] as? String)
        if read != level { level = read }
    }

    func probeState() -> [String: Any] {
        ["available": battery.available, "voltage": battery.voltageText,
         "current": battery.currentText, "percent": battery.percentText,
         "level": String(describing: level)]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
