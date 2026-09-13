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

    func refresh() {
        let read = MavlinkConsole(view: Bridge.group("view.mavlinkConsole"))
        if read != console { console = read }
    }

    // No send action, and there must never be one. sendCommand puts a string on the vehicle's
    // shell, which can reboot it, rewrite its parameters or arm it. Probeable.probeInvoke
    // defaults to refusing every action, which is what keeps that true by construction.
    func probeState() -> [String: Any] {
        ["connected": console.connected, "lines": console.lines.count,
         "last": console.last ?? "", "watching": watchPoll != nil,
         "emptyText": console.emptyText]
    }
}
