import Foundation

struct RemoteSupport: Equatable {
    let host: String
    let forwarding: Bool

    static let empty = RemoteSupport(host: "", forwarding: false)

    var canConnect: Bool { !forwarding && !host.isEmpty }

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
