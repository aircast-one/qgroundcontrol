import Foundation

enum VehicleSetupText {
    static func connectPrompt(for subject: String) -> String {
        "Connect a vehicle to see its \(subject)."
    }

    static func waiting(connected: Bool, for subject: String) -> String {
        connected
            ? "Reading \(subject) from the vehicle\u{2026}"
            : connectPrompt(for: subject)
    }

    // waiting() has two outcomes and the parameter load has three: still asking, gave up, and no
    // vehicle. QGC tries FTP, falls back to five conventional retries and then announces the
    // failure only through a showAppMessage dialog, so every setup screen here said "Reading
    // parameters from the vehicle..." for ever against a load that had stopped. The core answers
    // the third state now (setup.rs parameter_state) and this is where the three become sentences.
    //
    // The served text says what the VEHICLE did; the second sentence is what the OPERATOR can do,
    // and it is the head's to write because only the head knows what its own screens offer. That
    // route is checked rather than assumed: ConnectionsSection.swift:125 draws a per-link button
    // reading Disconnect then Connect, and a reconnect builds a fresh ParameterManager. There is
    // no route to refreshAllParameters here, so "ask again" on its own would name an action this
    // app cannot perform.
    static let reconnectToAsk = "Setup needs them. Disconnect and connect the link again to ask."

    static func parameters(reason: String, served: String) -> String {
        switch reason {
        case "": return ""
        case "noVehicle": return connectPrompt(for: "parameters")
        case "loading": return waiting(connected: true, for: "parameters")
        // A reason this head does not know must not read as "ready" and must not read as "still
        // asking" either -- the first hides a stopped load behind silence, the second is the defect
        // being fixed. It falls through to whatever the core said, and to the stopped sentence only
        // when the core said nothing, because an unknown state is closer to stopped than to fine.
        default: return served.isEmpty ? reconnectToAsk : served + " " + reconnectToAsk
        }
    }

    static func absent(connected: Bool, _ whatTheVehicleLacks: String) -> String {
        connected
            ? "This vehicle \(whatTheVehicleLacks)"
            : "Connect a vehicle to set this up."
    }

    static func filtered(connected: Bool) -> String {
        connected
            ? "No parameter matches this filter."
            : connectPrompt(for: "parameters")
    }
}
