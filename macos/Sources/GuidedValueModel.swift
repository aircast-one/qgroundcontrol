import Foundation

struct GuidedValue: Equatable {
    let label: String
    let measure: Measure
    let minimum: Double
    let maximum: Double
    let initial: Double

    init?(label: String, measure: Measure, minimum: Double, maximum: Double, initial: Double) {
        guard minimum.isFinite, maximum.isFinite, initial.isFinite, maximum > minimum else {
            return nil
        }
        self.label = label
        self.measure = measure
        self.minimum = minimum
        self.maximum = maximum
        self.initial = Swift.min(Swift.max(initial, minimum), maximum)
    }

    func clamped(_ value: Double) -> Double {
        Swift.min(Swift.max(value, minimum), maximum)
    }

    func text(_ value: Double) -> String {
        measure.text(clamped(value))
    }

    static func takeoff(minimumAltitude: Double, maximumAltitude: Double,
                        measure: Measure) -> GuidedValue? {
        GuidedValue(label: "Height above launch", measure: measure,
                    minimum: minimumAltitude, maximum: maximumAltitude,
                    initial: minimumAltitude)
    }

    static func altitude(minimum: Double, maximum: Double, current: Double,
                         measure: Measure) -> GuidedValue? {
        GuidedValue(label: "Height above launch", measure: measure,
                    minimum: minimum, maximum: maximum, initial: current)
    }

    static func speed(maximum: Double, forwardFlight: Bool,
                      minimumAirspeed: Double, maximumAirspeed: Double,
                      measure: Measure) -> GuidedValue? {
        forwardFlight
            ? GuidedValue(label: "Airspeed", measure: measure,
                          minimum: minimumAirspeed, maximum: maximumAirspeed,
                          initial: (minimumAirspeed + maximumAirspeed) / 2)
            : GuidedValue(label: "Ground speed", measure: measure,
                          minimum: 0.1, maximum: maximum, initial: maximum / 2)
    }
}
