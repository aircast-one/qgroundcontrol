import Foundation

final class MavlinkConsoleStore: ObservableObject, Probeable {
    static let probeID = "console"

    @Published private(set) var console = MavlinkConsole.none

    private var watchPoll: Timer?

    func startWatching() {
        guard watchPoll == nil else { return }
        refresh()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    // Not Bridge.group("vehicle")["kind"]: that serialises the whole vehicle object, the largest
    // node in the tree, once a second to read one word. This asks the one question instead.
    func refresh() {
        let live = (Bridge.group("vehicles.activeVehicleAvailable")["value"] as? NSNumber)?
            .boolValue ?? false
        let read = MavlinkConsole(Bridge.group("mavlinkConsole"), connected: live)
        if read != console { console = read }
    }

    // No send action, and there must never be one. sendCommand puts a string on the vehicle's
    // shell, which can reboot it, rewrite its parameters or arm it. Probeable.probeInvoke
    // defaults to refusing every action, which is what keeps that true by construction.
    func probeState() -> [String: Any] {
        ["connected": console.connected, "lines": console.lines.count,
         "last": console.last ?? "", "watching": watchPoll != nil,
         "empty": console.emptyText]
    }
}
