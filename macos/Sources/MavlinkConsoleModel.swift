import Foundation

struct MavlinkConsole: Equatable {
    let lines: [String]
    let connected: Bool

    static let none = MavlinkConsole(lines: [], connected: false)

    init(lines: [String], connected: Bool) {
        self.lines = lines
        self.connected = connected
    }

    // view.mavlinkConsole answers `connected` and `lines` in ONE read. The store used to ask
    // vehicles.activeVehicleAvailable and mavlinkConsole separately, so a vehicle that dropped
    // between the two calls produced a console that was connected and empty, or disconnected
    // with output. Narrow, and there is no reason to keep it.
    init(view json: [String: Any]) {
        // The line still being assembled used to be dropped here. e099c6dd6 pops it in the view,
        // so this takes the list AS GIVEN and must keep doing so: a blank entry that is not the
        // trailing one is a blank line the vehicle actually printed, which is output and spacing
        // an operator can see. Filtering empties would read as a tidier version of the same trim
        // and would silently delete real output.
        lines = ((json["lines"] as? [Any]) ?? []).compactMap { $0 as? String }
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
    }

    var describes: Bool { !lines.isEmpty }

    // Shell output only appears in answer to a command, and this page cannot send one, so a
    // connected operator watching "nothing yet" is being told to wait for something that will
    // never arrive. Say where the commands go instead.
    // NOT view.mavlinkConsole's emptyReason, deliberately. The core spells the connected case
    // "The vehicle has not printed anything yet", which is true of the vehicle and wrong for
    // THIS page: shell output only appears in answer to a command, and this page cannot send
    // one. That sentence tells a connected operator to wait for something that will never
    // arrive -- the same defect I removed from My Location twice in one day. What a head can do
    // is the head's to say.
    var emptyText: String {
        connected
            ? "Nothing yet. The shell answers commands, and those are sent from the Qt build."
            : VehicleSetupText.connectPrompt(for: "console output")
    }

    var copyable: String { lines.joined(separator: "\n") }

    // What the probe reports as the console's summary. The view shows the tail by scrolling,
    // not by reading this.
    var last: String? { lines.last }

    // Not built: the command input. sendCommand puts a string on the vehicle's shell, which can
    // do anything a shell can - reboot it, rewrite its parameters, arm it. Nothing here can send
    // one to check the wiring, and a gate that fails closed may ship unverified while a
    // capability that fails by executing may not.
    static let readOnlyNotice = "Reading only. Commands are sent from the Qt build."
}
