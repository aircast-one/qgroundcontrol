import Foundation

final class GuidedStore: ObservableObject, Probeable {
    static let probeID = "guided"

    @Published private(set) var state = GuidedState()
    @Published private(set) var pending: GuidedAction?
    @Published private(set) var lastSent = ""

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
        pending = action
    }

    func cancel() {
        pending = nil
    }

    func confirm() {
        guard let action = pending else { return }
        send(action)
        pending = nil
    }

    private func send(_ action: GuidedAction) {
        lastSent = action.rawValue
        switch action {
        case .arm: _ = Bridge.set("vehicle.armed", true)
        case .disarm: _ = Bridge.set("vehicle.armed", false)
        case .rtl: Bridge.invoke("vehicle.guidedModeRTL", [false])
        case .land: Bridge.invoke("vehicle.guidedModeLand")
        case .takeoff: Bridge.invoke("vehicle.guidedModeTakeoff", [takeoffAltitude])
        case .startMission, .continueMission: Bridge.invoke("vehicle.startMission")
        case .pause: Bridge.invoke("vehicle.pauseVehicle")
        case .emergencyStop: Bridge.invoke("vehicle.emergencyStop")
        }
    }

    private var takeoffAltitude: Double {
        let fact = Bridge.group("settings.appSettings.defaultMissionItemAltitude")
        return (fact["value"] as? NSNumber)?.doubleValue ?? 10
    }

    func probeState() -> [String: Any] {
        ["connected": state.connected, "armed": state.armed, "flying": state.flying,
         "flightMode": state.flightMode, "readyToArm": state.readyToArm,
         "missionActive": state.missionActive, "lastSent": lastSent,
         "pending": pending?.rawValue ?? "",
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
        case "cancel": cancel()
        default: return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
