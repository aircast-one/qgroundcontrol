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
        let frame = Bridge.group("view.frame")
        guard (frame["connected"] as? NSNumber)?.boolValue == true else {
            if setup != .unknown { setup = .unknown }
            return
        }

        // vehicleTypeText, not vehicleType: the token is the untranslated class and the text is the
        // tr() spelling an operator reads. The token is null while disconnected rather than
        // "Generic", because Generic is a vehicle QGC could not classify and no vehicle at all is a
        // different state -- this guard returns before either is consulted.
        let reading = FrameSetup(
            reportedType: (frame["vehicleTypeText"] as? String) ?? "",
            motorCount: (frame["motorCount"] as? NSNumber)?.intValue)
        if reading != setup { setup = reading }
    }

    func probeState() -> [String: Any] {
        ["known": setup.known, "vehicleTypeText": setup.vehicleTypeText,
         "motors": setup.motorText]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "refresh" else {
            return ["ok": false, "error": "unknown action \(action)"]
        }
        refresh()
        return ["ok": true, "state": probeState()]
    }
}
