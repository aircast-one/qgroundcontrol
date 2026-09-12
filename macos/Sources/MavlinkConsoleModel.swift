import Foundation

struct MavlinkConsole: Equatable {
    let lines: [String]
    let connected: Bool

    static let none = MavlinkConsole(lines: [], connected: false)

    init(lines: [String], connected: Bool) {
        self.lines = lines
        self.connected = connected
    }

    init(_ json: [String: Any], connected: Bool) {
        // MAVLinkConsoleController keeps a QStringList whose last entry is the line still being
        // assembled, so it is empty until a newline arrives and drawing it adds a blank row that
        // appears and disappears as characters land.
        let listed = ((json["lines"] as? [Any]) ?? []).compactMap { $0 as? String }
        lines = listed.last?.isEmpty == true ? Array(listed.dropLast()) : listed
        self.connected = connected
    }

    var describes: Bool { !lines.isEmpty }

    // Shell output only appears in answer to a command, and this page cannot send one, so a
    // connected operator watching "nothing yet" is being told to wait for something that will
    // never arrive. Say where the commands go instead.
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
