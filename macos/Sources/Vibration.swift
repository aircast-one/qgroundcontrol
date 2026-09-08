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
        let read = VibrationReading(Bridge.group("view.vibration"))
        if read != reading { reading = read }
    }

    func probeState() -> [String: Any] {
        ["available": reading.available, "units": reading.units,
         "axes": reading.axes.map { ["axis": $0.axis, "label": $0.label,
                                     "value": $0.value ?? -1,
                                     "severity": $0.severity.map { String(describing: $0) } ?? ""] },
         "clipCounts": reading.clipCounts, "clipping": reading.clipping,
         "worst": reading.worst.map { String(describing: $0) } ?? ""]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
