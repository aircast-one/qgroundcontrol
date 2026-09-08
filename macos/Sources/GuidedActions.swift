import Foundation

final class GuidedStore: ObservableObject, Probeable {
    static let probeID = "guided"

    @Published private(set) var state = GuidedState()
    @Published private(set) var pending: GuidedAction?
    @Published private(set) var lastSent = ""
    @Published private(set) var range: GuidedValue?
    @Published var chosen = 0.0

    var actions: [GuidedAction] { GuidedAction.offered(in: state) }

    func offer(_ action: GuidedAction) -> GuidedAction.Offer { action.offer(in: state) }

    func refresh(prearmClear: Bool) {
        let vehicle = Bridge.group("vehicle")
        guard vehicle["kind"] as? String == "object" else {
            if state != GuidedState() { state = GuidedState() }
            return
        }

        func flag(_ name: String) -> Bool {
            (vehicle[name] as? NSNumber)?.boolValue ?? false
        }

        let mission = Bridge.group("plan.missionController")
        var read = GuidedState()
        read.connected = true
        read.armed = flag("armed")
        read.flying = flag("flying")
        read.guidedSupported = flag("guidedModeSupported")
        read.takeoffSupported = flag("takeoffVehicleSupported")
        read.pauseSupported = flag("pauseVehicleSupported")
        read.fixedWing = flag("fixedWing")
        read.forwardFlight = flag("vtolInFwdFlight") || flag("fixedWing")
        read.speedLimitsAvailable = read.forwardFlight
            ? flag("haveFWSpeedLimits")
            : flag("haveMRSpeedLimits")
        read.landing = flag("landing")
        read.readyToArm = prearmClear
        read.flightMode = (vehicle["flightMode"] as? String) ?? ""
        read.rtlMode = (vehicle["rtlFlightMode"] as? String) ?? ""
        read.landMode = (vehicle["landFlightMode"] as? String) ?? ""
        read.missionMode = (vehicle["missionFlightMode"] as? String) ?? ""
        read.missionAvailable = (mission["containsItems"] as? NSNumber)?.boolValue ?? false
        read.missionItemCount = (mission["missionItemCount"] as? NSNumber)?.intValue ?? 0
        read.currentMissionIndex = (mission["currentMissionIndex"] as? NSNumber)?.intValue ?? -1

        if read != state { state = read }
        if let pending, !pending.available(in: read) { self.pending = nil }
    }

    func ask(_ action: GuidedAction) {
        guard action.available(in: state) else { return }
        let built = action.carriesValue ? limits(for: action) : nil
        guard !action.carriesValue || built != nil else { return }
        range = built
        chosen = built?.initial ?? 0
        pending = action
    }

    func cancel() {
        pending = nil
        range = nil
    }

    func confirm() {
        guard let action = pending else { return }
        guard !action.carriesValue || range != nil else { return }
        send(action)
        pending = nil
        range = nil
    }

    private func limits(for action: GuidedAction) -> GuidedValue? {
        switch action {
        case .takeoff:
            return GuidedValue.takeoff(minimumAltitude: number("vehicle.minimumTakeoffAltitudeMeters"),
                                       maximumAltitude: setting("guidedMaximumAltitude"))
        case .changeAltitude, .pause:
            return GuidedValue.altitude(minimum: setting("guidedMinimumAltitude"),
                                        maximum: setting("guidedMaximumAltitude"),
                                        current: currentAltitude)
        case .changeSpeed:
            return GuidedValue.speed(maximum: number("vehicle.maximumHorizontalSpeedMultirotor"),
                                     forwardFlight: state.forwardFlight,
                                     minimumAirspeed: number("vehicle.minimumEquivalentAirspeed"),
                                     maximumAirspeed: number("vehicle.maximumEquivalentAirspeed"))
        default:
            return nil
        }
    }

    static let climbOutAltitude = 50.0

    private func number(_ path: String) -> Double {
        (Bridge.invoke(path)["result"] as? NSNumber)?.doubleValue ?? .nan
    }

    private func setting(_ name: String) -> Double {
        metres("settings.flyViewSettings.\(name).rawValue")
    }

    private var currentAltitude: Double {
        metres("vehicle.altitudeRelative.rawValue")
    }

    private func metres(_ path: String) -> Double {
        (Bridge.group(path)["value"] as? NSNumber)?.doubleValue ?? .nan
    }

    private func send(_ action: GuidedAction) {
        lastSent = action.rawValue
        switch action {
        case .arm: _ = Bridge.set("vehicle.armed", true)
        case .disarm: _ = Bridge.set("vehicle.armed", false)
        case .rtl: Bridge.invoke("vehicle.guidedModeRTL", [false])
        case .land: Bridge.invoke("vehicle.guidedModeLand")
        case .takeoff: Bridge.invoke("vehicle.guidedModeTakeoff", [chosen])
        case .changeAltitude:
            Bridge.invoke("vehicle.guidedModeChangeAltitude", [chosen - currentAltitude, false])
        case .changeSpeed:
            Bridge.invoke(state.forwardFlight
                ? "vehicle.guidedModeChangeEquivalentAirspeedMetersSecond"
                : "vehicle.guidedModeChangeGroundSpeedMetersSecond", [chosen])
        case .startMission, .continueMission: Bridge.invoke("vehicle.startMission")
        case .pause:
            Bridge.invoke("vehicle.guidedModeChangeAltitude", [chosen - currentAltitude, true])
        case .landAbort: Bridge.invoke("vehicle.abortLanding", [GuidedStore.climbOutAltitude])
        case .emergencyStop: Bridge.invoke("vehicle.emergencyStop")
        }
    }

    func probeState() -> [String: Any] {
        ["connected": state.connected, "armed": state.armed, "flying": state.flying,
         "flightMode": state.flightMode, "readyToArm": state.readyToArm,
         "missionActive": state.missionActive, "lastSent": lastSent,
         "pending": pending?.rawValue ?? "",
         "range": range.map {
             ["label": $0.label, "units": $0.units, "min": $0.minimum,
              "max": $0.maximum, "initial": $0.initial]
         } ?? [:],
         "chosen": range.map { $0.text(chosen) } ?? "",
         "offered": actions.map(\.rawValue),
         "available": GuidedAction.available(in: state).map(\.rawValue)]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh(prearmClear: args["ready"] == "1")
        case "ask":
            guard let wanted = GuidedAction(rawValue: args["what"] ?? "") else {
                return ["ok": false, "error": "no action \(args["what"] ?? "")"]
            }
            guard wanted.available(in: state) else {
                return ["ok": false, "error": "\(wanted.title) is not available in this state"]
            }
            ask(wanted)
            guard pending == wanted else {
                return ["ok": false,
                        "error": "\(wanted.title) needs a range the vehicle has not reported"]
            }
        case "cancel": cancel()
        case "choose":
            guard let range, let wanted = Double(args["value"] ?? "") else {
                return ["ok": false, "error": "choose needs a value and an action that takes one"]
            }
            chosen = range.clamped(wanted)
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
