import Foundation

final class SensorsStore: ObservableObject, Probeable {
    static let probeID = "sensors"

    @Published private(set) var sensors: [SensorHealth] = []
    @Published private(set) var status = ""

    private var timer: Timer?

    var failing: [SensorHealth] { sensors.filter { $0.state == .unhealthy } }

    func start() {
        guard timer == nil else { return }
        refresh()
        let timer = Timer(timeInterval: 1.0, repeats: true) { [weak self] _ in self?.refresh() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    func refresh() {
        let json = Bridge.group("vehicle.sysStatusSensorInfo")
        let parsed = SensorHealth.from(json: json)
        sensors = SensorHealth.ordered(parsed)
        status = parsed.isEmpty ? "No vehicle is reporting sensor status." : ""
    }

    func probeState() -> [String: Any] {
        ["count": sensors.count,
         "failing": failing.map(\.name),
         "status": status,
         "order": sensors.prefix(6).map(\.name)]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
