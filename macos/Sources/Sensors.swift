import Foundation

final class SensorsStore: ObservableObject, Probeable {
    static let probeID = "sensors"

    @Published private(set) var sensors: [SensorHealth] = []
    @Published private(set) var status = ""
    @Published private(set) var calibration = CalibrationState.disconnected
    @Published private(set) var lastStarted = ""

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

        let read = Calibration.read(Bridge.group("sensorsCal"))
        if read != calibration { calibration = read }
    }

    func start(_ routine: CalibrationRoutine) {
        guard calibration.connected, !calibration.busy,
              !routine.blocked(whenAccelNeeded: calibration.accelNeeded) else { return }
        lastStarted = routine.rawValue
        Bridge.invoke(routine.invocation, routine.arguments)
        refresh()
    }

    func next() {
        guard calibration.nextEnabled else { return }
        Bridge.invoke("sensorsCal.nextClicked")
        refresh()
    }

    func cancelCalibration() {
        guard calibration.cancelEnabled else { return }
        Bridge.invoke("sensorsCal.cancelCalibration")
        refresh()
    }

    func probeState() -> [String: Any] {
        ["count": sensors.count,
         "calibration": ["connected": calibration.connected, "inProgress": calibration.inProgress,
                         "progress": calibration.progressText, "help": calibration.helpText,
                         "statusText": calibration.statusText, "busy": calibration.busy,
                         "needs": calibration.needsAttention,
                         "blocked": CalibrationRoutine.allCases
                             .filter { $0.blocked(whenAccelNeeded: calibration.accelNeeded) }
                             .map(\.rawValue),
                         "sides": calibration.visibleSides.map(\.title),
                         "lastStarted": lastStarted],
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
