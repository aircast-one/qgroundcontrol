import Foundation

final class SensorsStore: ObservableObject, Probeable {
    static let probeID = "sensors"

    @Published private(set) var sensors: [SensorHealth] = []
    @Published private(set) var status = ""
    @Published private(set) var calibration = CalibrationState.disconnected
    @Published private(set) var lastStarted = ""

    private var timer: Timer?

    @Published private(set) var failing: [String] = []

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
        let view = Bridge.group("view.sensors")
        let listed = SensorHealth.list(view["sensors"])
        if listed != sensors { sensors = listed }
        let faults = ((view["failing"] as? [Any]) ?? []).compactMap { $0 as? String }
        if faults != failing { failing = faults }
        let reported = (view["status"] as? String) ?? ""
        if reported != status { status = reported }

        let read = CalibrationState(Bridge.group("view.calibration"))
        if read != calibration { calibration = read }
    }

    func start(_ routine: CalibrationRoutine) {
        guard routine.enabled else { return }
        lastStarted = routine.id
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
                         "blocked": calibration.routines.filter(\.blocked).map(\.id),
                         "sides": calibration.visibleSides.map(\.title),
                         "lastStarted": lastStarted],
         "failing": failing,
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
