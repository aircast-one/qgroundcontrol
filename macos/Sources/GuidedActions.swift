import Foundation

final class GuidedStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "guided"

    @Published private(set) var offers: [GuidedOffer] = []
    @Published private(set) var connected = false
    @Published private(set) var missionActive = false
    @Published private(set) var pending: GuidedOffer?
    @Published private(set) var lastSent = ""
    @Published private(set) var range: GuidedValue?
    @Published var chosen = 0.0

    var actions: [GuidedOffer] { offers.filter(\.shown) }

    func refresh() {
        let view = Bridge.group("view.guidedActions")
        let read = GuidedOffer.list(view["actions"])
        if read != offers { offers = read }
        let live = (view["connected"] as? NSNumber)?.boolValue ?? false
        if live != connected { connected = live }
        let active = (view["missionActive"] as? NSNumber)?.boolValue ?? false
        if active != missionActive { missionActive = active }
        if let pending, !(offers.first { $0.id == pending.id }?.ready ?? false) { self.pending = nil }
    }

    func ask(_ offer: GuidedOffer) {
        guard offer.ready else { return }
        let built = offer.carriesValue ? limits(for: offer.action) : nil
        guard !offer.carriesValue || built != nil else { return }
        range = built
        chosen = built?.initial ?? 0
        pending = offer
    }

    func cancel() {
        pending = nil
        range = nil
    }

    func confirm() {
        guard let action = pending else { return }
        guard !action.carriesValue || range != nil else { return }
        send(action.action)
        pending = nil
        range = nil
    }

    private func limits(for action: GuidedAction) -> GuidedValue? {
        switch action {
        case .takeoff:
            return GuidedValue.takeoff(minimumAltitude: number("vehicle.minimumTakeoffAltitudeMeters"),
                                       maximumAltitude: setting("guidedMaximumAltitude"),
                                       measure: AppUnits.measure(AppUnits.vertical))
        case .changeAltitude, .pause:
            return GuidedValue.altitude(minimum: setting("guidedMinimumAltitude"),
                                        maximum: setting("guidedMaximumAltitude"),
                                        current: currentAltitude,
                                        measure: AppUnits.measure(AppUnits.vertical))
        case .changeSpeed:
            return GuidedValue.speed(maximum: number("vehicle.maximumHorizontalSpeedMultirotor"),
                                     forwardFlight: forwardFlight,
                                     minimumAirspeed: number("vehicle.minimumEquivalentAirspeed"),
                                     maximumAirspeed: number("vehicle.maximumEquivalentAirspeed"),
                                     measure: AppUnits.measure(AppUnits.speed))
        default:
            return nil
        }
    }

    static let climbOutAltitude = 50.0
    static let gripperRelease = 0
    static let gripperGrab = 1

    private var forwardFlight: Bool {
        let vehicle = Bridge.group("vehicle")
        func flag(_ name: String) -> Bool { (vehicle[name] as? NSNumber)?.boolValue ?? false }
        return flag("vtolInFwdFlight") || flag("fixedWing")
    }

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
        case .arm: write("vehicle.armed", true, "the vehicle to armed")
        case .disarm: write("vehicle.armed", false, "the vehicle to disarmed")
        case .rtl: Bridge.invoke("vehicle.guidedModeRTL", [false])
        case .land: Bridge.invoke("vehicle.guidedModeLand")
        case .takeoff: Bridge.invoke("vehicle.guidedModeTakeoff", [chosen])
        case .changeAltitude:
            Bridge.invoke("vehicle.guidedModeChangeAltitude", [chosen - currentAltitude, false])
        case .changeSpeed:
            Bridge.invoke(forwardFlight
                ? "vehicle.guidedModeChangeEquivalentAirspeedMetersSecond"
                : "vehicle.guidedModeChangeGroundSpeedMetersSecond", [chosen])
        case .startMission, .continueMission: Bridge.invoke("vehicle.startMission")
        case .pause:
            Bridge.invoke("vehicle.guidedModeChangeAltitude", [chosen - currentAltitude, true])
        case .landAbort: Bridge.invoke("vehicle.abortLanding", [GuidedStore.climbOutAltitude])
        case .emergencyStop: Bridge.invoke("vehicle.emergencyStop")
        case .grab: Bridge.invoke("vehicle.sendGripperAction", [GuidedStore.gripperGrab])
        case .release: Bridge.invoke("vehicle.sendGripperAction", [GuidedStore.gripperRelease])
        }
    }

    func probeState() -> [String: Any] {
        ["writeFailure": writeFailure ?? "",
         "connected": connected, "missionActive": missionActive, "lastSent": lastSent,
         "pending": pending?.id ?? "",
         "range": range.map {
             ["label": $0.label, "units": $0.measure.suffix, "min": $0.minimum,
              "max": $0.maximum, "initial": $0.initial]
         } ?? [:],
         "chosen": range.map { $0.text(chosen) } ?? "",
         "offered": actions.map(\.id),
         "available": actions.filter(\.ready).map(\.id),
         "blocked": Dictionary(uniqueKeysWithValues:
             actions.filter(\.blocked).map { ($0.id, $0.reason) })]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "ask":
            guard let wanted = offers.first(where: { $0.id == args["what"] }) else {
                return ["ok": false, "error": "no action \(args["what"] ?? "")"]
            }
            guard wanted.ready else {
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
