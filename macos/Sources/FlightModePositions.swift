import Foundation

// ArduPilot reads one RC channel and maps six PWM bands onto FLTMODE1..6. The bands are
// fixed in firmware, not parameters, so an operator matching their transmitter's switch
// to a mode has no way to see them unless the page states them.
struct FlightModePosition: Identifiable {
    let index: Int
    let parameter: String
    let pwmRange: String

    var id: Int { index }

    static let all: [FlightModePosition] = [
        FlightModePosition(index: 1, parameter: "FLTMODE1", pwmRange: "up to 1230"),
        FlightModePosition(index: 2, parameter: "FLTMODE2", pwmRange: "1231 – 1360"),
        FlightModePosition(index: 3, parameter: "FLTMODE3", pwmRange: "1361 – 1490"),
        FlightModePosition(index: 4, parameter: "FLTMODE4", pwmRange: "1491 – 1620"),
        FlightModePosition(index: 5, parameter: "FLTMODE5", pwmRange: "1621 – 1749"),
        FlightModePosition(index: 6, parameter: "FLTMODE6", pwmRange: "1750 and above"),
    ]

    static let channelParameter = "FLTMODE_CH"

    // Only the positions this vehicle actually reports, so a firmware that names them
    // differently shows nothing rather than six broken rows.
    static func present(in available: Set<String>) -> [FlightModePosition] {
        all.filter { available.contains($0.parameter) }
    }
}
