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

    static func present(in available: Set<String>) -> [FlightModePosition] {
        guard let naming = FlightModeNaming.chosen(from: available) else { return [] }
        return all.map { $0.named(by: naming) }.filter { available.contains($0.parameter) }
    }

    func named(by naming: FlightModeNaming) -> FlightModePosition {
        FlightModePosition(index: index, parameter: "\(naming.slotPrefix)\(index)", pwmRange: pwmRange)
    }
}

// APMFlightModesComponentController.cc:26-28 picks the prefix by asking whether MODE1 exists:
// a rover or boat names these MODE1..6 and MODE_CH, everything else FLTMODE1..6 and FLTMODE_CH.
// The head needs the NAMES rather than the core's slot text because each row edits the parameter.
struct FlightModeNaming: Equatable {
    let slotPrefix: String
    let channelParameter: String

    static let rover = FlightModeNaming(slotPrefix: "MODE", channelParameter: "MODE_CH")
    static let flying = FlightModeNaming(slotPrefix: "FLTMODE", channelParameter: "FLTMODE_CH")

    static func chosen(from available: Set<String>) -> FlightModeNaming? {
        if available.contains("MODE1") { return rover }
        if available.contains("FLTMODE1") { return flying }
        return nil
    }
}
