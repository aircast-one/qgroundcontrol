import Foundation

final class FrameStore: ObservableObject, Probeable {
    static let probeID = "frame"

    @Published private(set) var setup = FrameSetup.unknown

    private var timer: Timer?

    func start() {
        guard timer == nil else { return }
        refresh()
        let timer = Timer(timeInterval: 1, repeats: true) { [weak self] _ in self?.refresh() }
        RunLoop.main.add(timer, forMode: .common)
        self.timer = timer
    }

    func stop() {
        timer?.invalidate()
        timer = nil
    }

    func refresh() {
        let vehicle = Bridge.group("vehicle")
        guard vehicle["kind"] as? String == "object" else {
            if setup != .unknown { setup = .unknown }
            return
        }

        let reading = FrameSetup(
            vehicleType: (vehicle["vehicleTypeString"] as? String) ?? "",
            motorCount: (vehicle["motorCount"] as? NSNumber)?.intValue ?? 0)
        if reading != setup { setup = reading }
    }

    func probeState() -> [String: Any] {
        ["known": setup.known, "vehicleType": setup.vehicleTypeText, "motors": setup.motorText]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
