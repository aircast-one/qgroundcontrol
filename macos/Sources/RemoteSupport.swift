import Foundation

final class RemoteSupportStore: ObservableObject, Probeable {
    static let probeID = "remoteSupport"

    @Published private(set) var state = RemoteSupport.empty
    @Published var writeFailure: String?

    private static let hostPath = "settings.mavlinkSettings.forwardMavlinkAPMSupportHostName"

    func refresh() {
        let read = RemoteSupport(
            host: (Bridge.group(RemoteSupportStore.hostPath)["valueString"] as? String) ?? "",
            forwarding: (Bridge.group("links")["mavlinkSupportForwardingEnabled"] as? NSNumber)?
                .boolValue ?? false)
        if read != state { state = read }
    }

    func setHost(_ host: String) {
        guard !host.isEmpty else { return }
        if !Bridge.set(RemoteSupportStore.hostPath, host) {
            writeFailure = WriteReport.failure("the support host")
        }
        refresh()
    }

    func connect() {
        guard state.canConnect else { return }
        Bridge.invoke("links.createMavlinkForwardingSupportLink")
        refresh()
    }

    func probeState() -> [String: Any] {
        ["host": state.host, "forwarding": state.forwarding,
         "canConnect": state.canConnect, "status": state.status,
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
