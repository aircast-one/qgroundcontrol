import Foundation

struct MotorTest: Equatable {
    static let fallbackMotors = 8
    static let timeoutSeconds = 3
    static let minimumThrottle = 0.0
    static let maximumThrottle = 100.0

    // ABSENT, not a sentinel. view.frame answers null where QGC answers -1 -- fixed wing, rover,
    // boat, airship, anything it does not enumerate -- and null for a submarine until its
    // parameters arrive, where motorCount reads a confident 6 off a default fact. That 6 is the
    // case a sign test cannot see: it is positive, plausible, and nobody reported it.
    // The sentinel this replaces was -1, which is also what QGC returns, and no fixture built from
    // it could tell a sign test from an equality one.
    let reportedCount: Int?
    let letterIndices: Bool
    let connected: Bool
    let armed: Bool

    static let disconnected = MotorTest(reportedCount: nil, letterIndices: false,
                                        connected: false, armed: false)

    // The sign test stays alongside the absence: the core filters -1 but nothing makes a served 0
    // impossible, and a zero motor count would otherwise draw an empty grid with no warning. Both
    // answers mean the same thing to an operator -- the vehicle has not said.
    var countKnown: Bool { (reportedCount ?? 0) > 0 }

    var motors: Int { countKnown ? (reportedCount ?? 0) : MotorTest.fallbackMotors }

    var countWarning: String {
        connected && !countKnown
            ? "The vehicle has not said how many motors it has, so eight are offered."
            : ""
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

    // Stop is not a test and must not inherit a test's gate. stopAll() guards on connected alone --
    // correctly, since sending zero throttle needs nothing else -- while the button sat inside the
    // canTest group, so the two conditions that are REASONS TO STOP both disabled it: arm the
    // vehicle while the motors are turning, or drop the safety switch, and the one control an
    // operator reaches for greys out. A control is enabled by what its own action requires.
    var canStop: Bool { connected }

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
