import Foundation

struct MotorTest: Equatable {
    static let unknownCount = -1
    static let fallbackMotors = 8
    static let timeoutSeconds = 3
    static let minimumThrottle = 0.0
    static let maximumThrottle = 100.0

    let reportedCount: Int
    let letterIndices: Bool
    let connected: Bool
    let armed: Bool

    static let disconnected = MotorTest(reportedCount: unknownCount, letterIndices: false,
                                        connected: false, armed: false)

    var countKnown: Bool { reportedCount > 0 }

    var motors: Int { countKnown ? reportedCount : MotorTest.fallbackMotors }

    var countWarning: String {
        countKnown ? "" : "The vehicle has not said how many motors it has, so eight are offered."
    }

    func name(_ index: Int) -> String {
        letterIndices
            ? String(UnicodeScalar(UInt8(65 + index)))
            : "\(index + 1)"
    }

    var names: [String] { (0..<motors).map(name) }

    var armedRefusal: String {
        armed ? "The vehicle is armed. Disarm it before testing a motor." : ""
    }

    func canTest(safetyOff: Bool) -> Bool { connected && safetyOff && !armed }

    static func timeout(throttle: Double) -> Int {
        throttle <= minimumThrottle ? 0 : timeoutSeconds
    }

    static func clamp(_ throttle: Double) -> Double {
        Swift.min(Swift.max(throttle, minimumThrottle), maximumThrottle)
    }

    static func safetyText(_ safetyOff: Bool) -> String {
        safetyOff
            ? "Careful: the motors will turn."
            : "Take the propellers off, then switch this on to enable the motors."
    }
}
