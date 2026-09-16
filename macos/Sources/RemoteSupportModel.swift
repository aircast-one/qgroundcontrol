import Foundation

struct RemoteSupport: Equatable {
    let host: String
    let forwarding: Bool

    static let empty = RemoteSupport(host: "", forwarding: false, valid: nil, error: "")

    // QGC ships the EXAMPLE as the value. Mavlink.SettingsGroup.json:54 defaults
    // forwardMavlinkAPMSupportHostName to "support.ardupilot.org:xxxx" -- the same string line 52
    // gives as "i.e:" -- so a factory install arrives here holding a host that is not a host, and
    // !host.isEmpty called it ready. Pressing Connect then SUCCEEDS quietly: addHost splits on ":"
    // and gets two parts, so "Invalid host format" never fires; "xxxx".toUInt() is 0, because
    // QString returns 0 on failure when no ok pointer is passed; the name resolves, so "Could not
    // resolve host" never fires either. A UDPClient is appended for port 0 and this page reports
    // "MAVLink is being forwarded." while the engineer waiting at the other end sees nothing.
    // A DEFAULT IS NOT AN EXPRESSED INTENTION -- and this one is not even a valid value.
    // The rule moved to the core (view.supportHost, links.rs:145) and it is a RULE RE-DERIVED FROM
    // QGC's C++ rather than a fact this head consumes, which is the category that drifts. Android's
    // read the text after the LAST colon and so called "host:" port-less and "::1" port 1 -- both
    // accepted, both refused by addHost. Mine agreed with QGC on those two rows by arithmetic on a
    // part count, not because I had read UDPLink.cc:160's three-or-more branch, which returns
    // WITHOUT APPENDING ANY HOST. True and under-evidenced is still a reason to stop keeping it.
    //
    // Bool? rather than Bool: an unanswered check must not read as "nothing wrong". The whole
    // defect being fixed here is a control that offered to dial a host nobody had judged, so
    // absent falls CLOSED -- canConnect asks for a served true, never for the absence of a false.
    let valid: Bool?
    let error: String

    static let unchecked = "The address has not been checked yet."

    var hostRefusal: String {
        guard let valid else { return RemoteSupport.unchecked }
        return valid ? "" : error
    }

    var canConnect: Bool { !forwarding && valid == true }

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
