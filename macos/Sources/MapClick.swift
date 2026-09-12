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
    @Published private(set) var openPoint = CGPoint.zero
    @Published var confirming: MapClickTarget?
    @Published private(set) var lastSent = ""
    @Published private(set) var overlays = FlyOverlays.none
    @Published private(set) var scaleBar = MapScaleBar.none
    private var goingTo: GeoPoint?

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
            goingTo = nil
            if overlays != .none { overlays = .none }
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
        // The whole view for one number, because a computed view has no sub-paths -
        // view.guidedActions.gotoLoiterRadius answers kind:null. Measured at 2851 bytes, smaller
        // than the vehicle object this same refresh already reads. GuidedStore reads this view
        // too; check the cost again before a third reader is added.
        read.gotoLoiterRadius = (Bridge.group("view.guidedActions")["gotoLoiterRadius"]
            as? NSNumber)?.doubleValue ?? 0
        read.confirmGotoInGuided = (Bridge.group(
            "settings.flyViewSettings.goToLocationRequiresConfirmInGuided")["value"] as? NSNumber)?
            .boolValue ?? true
        read.roiActive = flag("isROIEnabled")

        if read != state { state = read }

        if !FlyOverlays.keepsGoto(flightMode: mode,
                                  gotoFlightMode: (vehicle["gotoFlightMode"] as? String) ?? "") {
            goingTo = nil
        }
        var drawn = FlyOverlays.read(
            orbitCircle: vehicle["orbitMapCircle"] as? [String: Any],
            radius: (Bridge.group("view.orbit")["radiusMetres"] as? NSNumber)?.doubleValue ?? 0,
            orbitActive: flag("orbitActive"),
            roiActive: read.roiActive,
            roi: Bridge.group("vehicle.roiCoord"))
        drawn.goingTo = goingTo
        if drawn != overlays { overlays = drawn }

        let bar = MissionMap.lastScale["fly"] ?? .none
        if bar != scaleBar { scaleBar = bar }

        dismissIfUnavailable()
    }

    private func dismissIfUnavailable() {
        if let openAt, !openAt.action.shown(in: state), offered.isEmpty { self.openAt = nil }
        if let confirming, !confirming.action.shown(in: state) { self.confirming = nil }
    }

    var offered: [MapClickAction] { MapClickAction.offered(in: state) }

    func open(latitude: Double, longitude: Double, at point: CGPoint = .zero) {
        guard !offered.isEmpty, let first = offered.first else { return }
        openPoint = point
        openAt = MapClickTarget(action: first, latitude: latitude, longitude: longitude)
    }

    // The marker must never outlive the menu, and openAt is cleared in four places, so it
    // is derived here rather than assigned alongside each of them.
    var shownOverlays: FlyOverlays {
        var shown = overlays
        shown.clickedAt = openAt.map { GeoPoint(latitude: $0.latitude, longitude: $0.longitude) }
        return shown
    }

    func close() { openAt = nil }

    func stopLooking() {
        guard state.roiActive, MapClickAction.cancelRoi.shown(in: state) else { return }
        confirming = MapClickTarget(action: .cancelRoi, latitude: 0, longitude: 0)
    }

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
        case .goTo: Bridge.invoke(path, [coordinate, state.gotoLoiterRadius])
        default: Bridge.invoke(path, [coordinate])
        }
        if target.action == .goTo {
            goingTo = GeoPoint(latitude: target.latitude, longitude: target.longitude)
        }
        lastSent = target.action.rawValue
    }

    func probeState() -> [String: Any] {
        ["connected": state.connected, "flying": state.flying,
         "gotoLoiterRadius": state.gotoLoiterRadius,
         "roiSupported": state.roiSupported, "orbitSupported": state.orbitSupported,
         "homeUsable": state.homeUsable, "gpsSensorPresent": state.gpsSensorPresent,
         "missionActive": state.missionActive, "roiActive": state.roiActive,
         "inGotoMode": state.inGotoMode, "confirmGotoInGuided": state.confirmGotoInGuided,
         "offered": offered.map(\.title),
         "refusal": offered.isEmpty ? MapClickAction.refusal(in: state) : "",
         "menuOpen": openAt != nil,
         "menuAt": ["x": Double(openPoint.x), "y": Double(openPoint.y)],
         "confirming": confirming?.action.title ?? "",
         "lastSent": lastSent,
         "map": MissionMap.lastRender["fly"] ?? [:],
         "scale": scaleBar.text,
         "overlays": ["orbit": overlays.showsOrbit, "roi": overlays.roiActive,
                      "roiPlaced": overlays.showsRoi,
                      "goto": overlays.showsGoto, "summary": overlays.summary,
                      "roiNote": overlays.roiNote,
                      "clicked": shownOverlays.clickedAt != nil]]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "open":
            guard let latitude = Double(args["latitude"] ?? ""),
                  let longitude = Double(args["longitude"] ?? "") else {
                return ["ok": false, "error": "open needs latitude and longitude"]
            }
            open(latitude: latitude, longitude: longitude,
                 at: CGPoint(x: Double(args["x"] ?? "") ?? 0,
                             y: Double(args["y"] ?? "") ?? 0))
        case "close": close()
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
