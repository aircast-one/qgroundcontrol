import Foundation

struct RemoteSupport: Equatable {
    let host: String
    let forwarding: Bool

    static let empty = RemoteSupport(host: "", forwarding: false)

    // QGC ships the EXAMPLE as the value. Mavlink.SettingsGroup.json:54 defaults
    // forwardMavlinkAPMSupportHostName to "support.ardupilot.org:xxxx" -- the same string line 52
    // gives as "i.e:" -- so a factory install arrives here holding a host that is not a host, and
    // !host.isEmpty called it ready. Pressing Connect then SUCCEEDS quietly: addHost splits on ":"
    // and gets two parts, so "Invalid host format" never fires; "xxxx".toUInt() is 0, because
    // QString returns 0 on failure when no ok pointer is passed; the name resolves, so "Could not
    // resolve host" never fires either. A UDPClient is appended for port 0 and this page reports
    // "MAVLink is being forwarded." while the engineer waiting at the other end sees nothing.
    // A DEFAULT IS NOT AN EXPRESSED INTENTION -- and this one is not even a valid value.
    static let lowestPort = 1
    static let highestPort = 65535

    // A host with no colon is legitimate: addHost falls back to the link's own local port for it,
    // so refusing one would break a configuration QGC accepts. Only the port half is judged.
    static func refusal(host: String) -> String? {
        let trimmed = host.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return "Enter the host the support engineer gave you." }
        guard trimmed.contains(":") else { return nil }
        let parts = trimmed.split(separator: ":", omittingEmptySubsequences: false)
        guard parts.count == 2, !parts[0].isEmpty else {
            return "\(trimmed) is not a host name and port."
        }
        guard let port = Int(parts[1]),
              port >= RemoteSupport.lowestPort, port <= RemoteSupport.highestPort else {
            return "\(parts[1]) is not a port number. The support engineer gives you the port."
        }
        return nil
    }

    var hostRefusal: String { RemoteSupport.refusal(host: host) ?? "" }

    var canConnect: Bool { !forwarding && hostRefusal.isEmpty }

    // LinkManager used to latch this on: the flag was set true in one place and never cleared,
    // so the page said forwarding could only end with the app, and the documented escape - remove
    // the link by hand - left the page in a state it could not leave. The flag is now derived from
    // the live link and endMavlinkForwardingSupportLink ends it, so the page offers the way out.
    var canStop: Bool { forwarding }

    var actionTitle: String { forwarding ? "Stop" : "Connect" }

    var status: String {
        forwarding ? "MAVLink is being forwarded." : "Nothing is being forwarded."
    }

    var note: String {
        "Sends this vehicle's MAVLink traffic to the support host so someone there can watch the flight with you."
    }
}
