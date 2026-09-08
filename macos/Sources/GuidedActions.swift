import Foundation

final class GuidedStore: ObservableObject, Probeable, WriteReporting {
    @Published var writeFailure: String?
    static let probeID = "guided"

    @Published private(set) var offers: [GuidedOffer] = []
    @Published private(set) var connected = false
    @Published private(set) var missionActive = false
    @Published private(set) var pending: GuidedOffer?
    @Published private(set) var lastSent = ""
    @Published private(set) var range: GuidedRange?
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
        let built = offer.carriesValue ? range(for: offer.action) : nil
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

    private func valueView(_ action: GuidedAction, _ target: Double? = nil) -> [String: Any] {
        let argument = target.map { "(\($0))" } ?? ""
        switch action {
        case .takeoff: return Bridge.group("view.guidedTakeoff\(argument)")
        case .changeAltitude, .pause: return Bridge.group("view.guidedAltitude\(argument)")
        case .changeSpeed: return Bridge.group("view.guidedSpeed\(argument)")
        default: return [:]
        }
    }

    private func range(for action: GuidedAction) -> GuidedRange? {
        GuidedRange(valueView(action))
    }

    static let climbOutAltitude = 50.0
    static let gripperRelease = 0
    static let gripperGrab = 1

    private func send(_ action: GuidedAction) {
        lastSent = action.rawValue
        let answer = valueView(action, range == nil ? nil : chosen)
        switch action {
        case .arm: write("vehicle.armed", true, "the vehicle to armed")
        case .disarm: write("vehicle.armed", false, "the vehicle to disarmed")
        case .rtl: Bridge.invoke("vehicle.guidedModeRTL", [false])
        case .land: Bridge.invoke("vehicle.guidedModeLand")
        case .takeoff:
            guard let metres = answer["targetMeters"] as? NSNumber else { return }
            Bridge.invoke("vehicle.guidedModeTakeoff", [metres.doubleValue])
        case .changeAltitude, .pause:
            guard let delta = answer["deltaMeters"] as? NSNumber else { return }
            guard (answer["sends"] as? NSNumber)?.boolValue == true else {
                lastSent = ""
                writeFailure = (answer["sentence"] as? String) ?? ""
                return
            }
            Bridge.invoke("vehicle.guidedModeChangeAltitude",
                          [delta.doubleValue, action == .pause])
        case .changeSpeed:
            guard let command = answer["command"] as? String,
                  let metres = answer["targetMetersSecond"] as? NSNumber else { return }
            Bridge.invoke("vehicle.\(command)", [metres.doubleValue])
        case .startMission, .continueMission: Bridge.invoke("vehicle.startMission")
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
             ["label": $0.label, "units": $0.unit, "min": $0.minimum,
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
