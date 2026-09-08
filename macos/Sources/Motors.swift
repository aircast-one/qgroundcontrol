import Foundation

final class MotorsStore: ObservableObject, Probeable {
    static let probeID = "motors"

    @Published private(set) var state = MotorTest.disconnected
    @Published var safetyOff = false
    @Published var throttle = MotorTest.minimumThrottle
    @Published private(set) var lastTested = ""

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
        safetyOff = false
        throttle = MotorTest.minimumThrottle
    }

    func refresh() {
        let vehicle = Bridge.group("vehicle")
        let read = MotorTest(
            reportedCount: (vehicle["motorCount"] as? NSNumber)?.intValue ?? MotorTest.unknownCount,
            letterIndices: (vehicle["apmFirmware"] as? NSNumber)?.boolValue ?? false,
            connected: vehicle["kind"] as? String == "object",
            armed: (vehicle["armed"] as? NSNumber)?.boolValue ?? false)
        if read != state { state = read }
        if !state.canTest(safetyOff: safetyOff), safetyOff {
            safetyOff = false
            throttle = MotorTest.minimumThrottle
        }
    }

    func setSafety(_ on: Bool) {
        safetyOff = on
        if !on { throttle = MotorTest.minimumThrottle }
    }

    func test(motor index: Int) {
        guard state.canTest(safetyOff: safetyOff), index < state.motors else { return }
        run(motor: index, throttle: throttle)
        lastTested = state.name(index)
    }

    func testAll() {
        guard state.canTest(safetyOff: safetyOff) else { return }
        (0..<state.motors).forEach { run(motor: $0, throttle: throttle) }
        lastTested = "all"
    }

    func stopAll() {
        guard state.connected else { return }
        (0..<state.motors).forEach { run(motor: $0, throttle: MotorTest.minimumThrottle) }
        lastTested = "stop"
    }

    private func run(motor index: Int, throttle: Double) {
        let value = MotorTest.clamp(throttle)
        Bridge.invoke("vehicle.motorTest",
                      [index + 1, value, MotorTest.timeout(throttle: value), true])
    }

    func probeState() -> [String: Any] {
        ["connected": state.connected, "armed": state.armed,
         "motorCount": state.reportedCount, "motors": state.motors,
         "names": state.names, "countWarning": state.countWarning,
         "armedRefusal": state.armedRefusal,
         "safetyOff": safetyOff, "throttle": throttle,
         "canTest": state.canTest(safetyOff: safetyOff),
         "safetyText": MotorTest.safetyText(safetyOff),
         "lastTested": lastTested]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "safety": setSafety(args["on"] != "0")
        case "throttle":
            guard let value = Double(args["value"] ?? "") else {
                return ["ok": false, "error": "throttle needs a value"]
            }
            throttle = MotorTest.clamp(value)
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
