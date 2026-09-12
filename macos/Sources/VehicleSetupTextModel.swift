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
