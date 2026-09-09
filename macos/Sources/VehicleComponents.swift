import Foundation

final class VehicleComponentsStore: ObservableObject, Probeable {
    static let probeID = "vehicleComponents"

    @Published private(set) var components: [VehicleComponentInfo] = []
    @Published private(set) var connected = false
    @Published private(set) var readiness = VehicleReadiness.unknown
    @Published private(set) var groups: [SetupGroup] = []

    private var watchPoll: Timer?

    // Which pages exist depends on the firmware, so the list this window navigates by changes the
    // moment a vehicle connects.
    func startWatching() {
        guard watchPoll == nil else { return }
        reload()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.reload()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    private(set) var reloads = 0

    func reload() {
        reloads += 1
        let view = Bridge.group("view.setup")
        let live = (view["connected"] as? NSNumber)?.boolValue ?? false
        if live != connected { connected = live }
        let listed = VehicleComponentInfo.list(view["components"])
        if listed != components { components = listed }
        let read = VehicleReadiness(view)
        if read != readiness { readiness = read }
        let catalogue = SetupCatalogue.offered(SetupCatalogue.groups(view["groups"]))
        if !catalogue.isEmpty, catalogue != groups { groups = catalogue }
    }

    var pageNames: [String] { SetupCatalogue.names(groups) }

    var outstanding: [VehicleComponentInfo] { components.filter(\.needsAttention) }

    func probeState() -> [String: Any] {
        ["connected": connected, "count": components.count,
         "reloads": reloads, "watching": watchPoll != nil, "pages": pageNames,
         "outstanding": outstanding.map(\.name),
         "ready": readiness.ready, "headline": readiness.headline, "detail": readiness.detail,
         "components": components.map {
             ["name": $0.name, "needsAttention": $0.needsAttention]
         }]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        guard action == "reload" else { return ["ok": false, "error": "unknown action \(action)"] }
        reload()
        return ["ok": true, "state": probeState()]
    }
}
