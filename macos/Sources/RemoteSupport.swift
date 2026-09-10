import Foundation

final class RemoteSupportStore: ObservableObject, Probeable, WriteReporting {
    static let probeID = "remoteSupport"

    @Published private(set) var state = RemoteSupport.empty
    @Published var writeFailure: String?

    private var watchPoll: Timer?

    private static let hostPath = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName"

    // A one-shot read was only ever defensible because the flag could not change without a
    // restart. It can now - Stop ends it, and the link disconnects asynchronously so the value
    // is not settled when stop() returns - so the page watches while it is open. Removing the
    // latch is what invalidated the one-shot design.
    func startWatching() {
        guard watchPoll == nil else { return }
        refresh()
        watchPoll = Timer.scheduledTimer(withTimeInterval: 0.5, repeats: true) { [weak self] _ in
            self?.refresh()
        }
    }

    func stopWatching() {
        watchPoll?.invalidate()
        watchPoll = nil
    }

    func refresh() {
        let read = RemoteSupport(
            host: (Bridge.group(RemoteSupportStore.hostPath)["valueString"] as? String) ?? "",
            forwarding: (Bridge.group("links")["mavlinkSupportForwardingEnabled"] as? NSNumber)?
                .boolValue ?? false)
        if read != state { state = read }
    }

    func setHost(_ host: String) {
        guard !host.isEmpty else { return }
        write(RemoteSupportStore.hostPath, host, "the support host")
        refresh()
    }

    func connect() {
        guard state.canConnect else { return }
        Bridge.invoke("links.createMavlinkForwardingSupportLink")
        refresh()
    }

    // The head must be able to end what it started, so this is built; it is NOT wired to the
    // probe, because stopping would tear down forwarding another session may be running.
    func stop() {
        guard state.canStop else { return }
        Bridge.invoke("links.endMavlinkForwardingSupportLink")
        refresh()
    }

    func probeState() -> [String: Any] {
        ["host": state.host, "forwarding": state.forwarding,
         "canConnect": state.canConnect, "canStop": state.canStop,
         "action": state.actionTitle, "status": state.status,
         "watching": watchPoll != nil,
         "writeFailure": writeFailure ?? ""]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "refresh": refresh()
        case "setHost": setHost(args["host"] ?? "")
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}
