import Foundation

struct GuidedValue: Equatable {
    let label: String
    let units: String
    let minimum: Double
    let maximum: Double
    let initial: Double

    init?(label: String, units: String, minimum: Double, maximum: Double, initial: Double) {
        guard minimum.isFinite, maximum.isFinite, initial.isFinite, maximum > minimum else {
            return nil
        }
        self.label = label
        self.units = units
        self.minimum = minimum
        self.maximum = maximum
        self.initial = Swift.min(Swift.max(initial, minimum), maximum)
    }

    func clamped(_ value: Double) -> Double {
        Swift.min(Swift.max(value, minimum), maximum)
    }

    func text(_ value: Double) -> String {
        String(format: units == "m" ? "%.0f %@" : "%.1f %@", clamped(value), units)
    }

    static func takeoff(minimumAltitude: Double, maximumAltitude: Double) -> GuidedValue? {
        GuidedValue(label: "Height above launch", units: "m",
                    minimum: minimumAltitude, maximum: maximumAltitude,
                    initial: minimumAltitude)
    }

    static func altitude(minimum: Double, maximum: Double, current: Double) -> GuidedValue? {
        GuidedValue(label: "Height above launch", units: "m",
                    minimum: minimum, maximum: maximum, initial: current)
    }

    static func speed(maximum: Double, forwardFlight: Bool,
                      minimumAirspeed: Double, maximumAirspeed: Double) -> GuidedValue? {
        forwardFlight
            ? GuidedValue(label: "Airspeed", units: "m/s",
                          minimum: minimumAirspeed, maximum: maximumAirspeed,
                          initial: (minimumAirspeed + maximumAirspeed) / 2)
            : GuidedValue(label: "Ground speed", units: "m/s",
                          minimum: 0.1, maximum: maximum, initial: maximum / 2)
    }
}
