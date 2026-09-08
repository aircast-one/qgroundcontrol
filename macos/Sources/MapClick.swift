import Foundation

struct MapClickTarget: Equatable {
    let action: MapClickAction
    let latitude: Double
    let longitude: Double
}

final class MapClickStore: ObservableObject, Probeable {
    static let probeID = "mapClick"

    @Published private(set) var state = MapClickState()
    @Published var openAt: MapClickTarget?
    @Published var confirming: MapClickTarget?
    @Published private(set) var lastSent = ""

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
        openAt = nil
        confirming = nil
    }

    func refresh() {
        let vehicle = Bridge.group("vehicle")
        guard vehicle["kind"] as? String == "object" else {
            if state != MapClickState() { state = MapClickState() }
            dismissIfUnavailable()
            return
        }

        func flag(_ name: String) -> Bool { (vehicle[name] as? NSNumber)?.boolValue ?? false }

        let home = vehicle["homePosition"] as? [String: Any]
        let homeAltitude = (home?["altitude"] as? NSNumber)?.doubleValue
        let bits = (vehicle["sensorsPresentBits"] as? NSNumber)?.intValue ?? 0
        let mode = (vehicle["flightMode"] as? String) ?? ""

        var read = MapClickState()
        read.connected = true
        read.flying = flag("flying")
        read.missionActive = flag("armed")
            && [vehicle["landFlightMode"], vehicle["rtlFlightMode"], vehicle["missionFlightMode"]]
                .contains { ($0 as? String) == mode }
        read.roiSupported = flag("roiModeSupported")
        read.orbitSupported = flag("orbitModeSupported")
        read.homeUsable = MapClickState.homeUsable(GeoPoint(json: home), altitude: homeAltitude)
        read.gpsSensorPresent = bits & MapClickState.gpsSensorBit != 0
        read.inGotoMode = !mode.isEmpty && (vehicle["gotoFlightMode"] as? String) == mode
        read.confirmGotoInGuided = (Bridge.group(
            "settings.flyViewSettings.goToLocationRequiresConfirmInGuided")["value"] as? NSNumber)?
            .boolValue ?? true
        read.roiActive = flag("isROIEnabled")

        if read != state { state = read }
        dismissIfUnavailable()
    }

    private func dismissIfUnavailable() {
        if let openAt, !openAt.action.shown(in: state), offered.isEmpty { self.openAt = nil }
        if let confirming, !confirming.action.shown(in: state) { self.confirming = nil }
    }

    var offered: [MapClickAction] { MapClickAction.offered(in: state) }

    func open(latitude: Double, longitude: Double) {
        guard !offered.isEmpty, let first = offered.first else { return }
        openAt = MapClickTarget(action: first, latitude: latitude, longitude: longitude)
    }

    func close() { openAt = nil }

    func choose(_ action: MapClickAction) {
        guard let openAt, action.shown(in: state) else { return }
        let target = MapClickTarget(action: action, latitude: openAt.latitude,
                                    longitude: openAt.longitude)
        self.openAt = nil
        if action.needsConfirmation(in: state) {
            confirming = target
        } else {
            send(target)
        }
    }

    func cancel() { confirming = nil }

    func confirm() {
        guard let confirming else { return }
        self.confirming = nil
        send(confirming)
    }

    private func send(_ target: MapClickTarget) {
        let path = "vehicle.\(target.action.invokable)"
        let coordinate: [String: Any] = ["latitude": target.latitude,
                                         "longitude": target.longitude,
                                         "altitude": 0]
        switch target.action {
        case .cancelRoi: Bridge.invoke(path)
        case .orbit: Bridge.invoke(path, [coordinate, 0, 0])
        case .goTo: Bridge.invoke(path, [coordinate, 0])
        default: Bridge.invoke(path, [coordinate])
        }
        lastSent = target.action.rawValue
    }

    func probeState() -> [String: Any] {
        ["connected": state.connected, "flying": state.flying,
         "roiSupported": state.roiSupported, "orbitSupported": state.orbitSupported,
         "homeUsable": state.homeUsable, "gpsSensorPresent": state.gpsSensorPresent,
         "missionActive": state.missionActive, "roiActive": state.roiActive,
         "inGotoMode": state.inGotoMode, "confirmGotoInGuided": state.confirmGotoInGuided,
         "offered": offered.map(\.title),
         "refusal": offered.isEmpty ? MapClickAction.refusal(in: state) : "",
         "menuOpen": openAt != nil,
         "confirming": confirming?.action.title ?? "",
         "lastSent": lastSent]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "open":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "open needs latitude and longitude"]
            }
            open(latitude: latitude, longitude: longitude)
        case "close": close()
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
