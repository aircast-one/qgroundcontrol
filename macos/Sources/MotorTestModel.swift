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
    // ABSENT means NOBODY IS WATCHING, and that is neither lost nor fine. frame.rs serves
    // `connected.then(|| contact_lost(backend)).flatten()`, and its own test says why the raw flag
    // could not be used: "with the watch off the flag stays false however long the vehicle has been
    // silent, so serving it raw would call an unmonitored link healthy on the page that decides
    // whether a motor may spin". So a null must not refuse -- refusing on unknown would ground a
    // bench test whenever link monitoring happens to be off -- and it must not reassure either,
    // which is why it is Bool? here rather than a defaulted false.
    let contactLost: Bool?

    static let disconnected = MotorTest(reportedCount: nil, letterIndices: false,
                                        connected: false, armed: false, contactLost: nil)

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

    // KNOWN lost, not merely not-known-good. `contactLost == true` is the only refusing value.
    var contactKnownLost: Bool { contactLost == true }

    var contactRefusal: String {
        contactKnownLost ? "The vehicle has stopped answering. Check the link before testing a motor." : ""
    }

    func canTest(safetyOff: Bool) -> Bool {
        connected && safetyOff && !armed && !contactKnownLost
    }

    // Stop is not a test and must not inherit a test's gate. stopAll() guards on connected alone --
    // correctly, since sending zero throttle needs nothing else -- while the button sat inside the
    // canTest group, so the two conditions that are REASONS TO STOP both disabled it: arm the
    // vehicle while the motors are turning, or drop the safety switch, and the one control an
    // operator reaches for greys out. A control is enabled by what its own action requires.
    var canStop: Bool { connected }

    // The safety toggle deliberately does NOT inherit canTest, and the difference is the point:
    // it stays reachable when contact is known lost. Flipping it sends nothing -- every action it
    // unlocks is gated by canTest, which does refuse on lost contact -- so disabling it here would
    // stop an operator arming the switch while waiting for the link to come back, and gain no
    // safety. It does refuse while ARMED, because that is the state the switch exists to keep you
    // out of. This lived as .disabled(!connected || armed) in VehicleSetupWindow, which
    // swift-checks does not compile, so the divergence from canTest was a coincidence of two
    // expressions rather than a decision anything held to.
    var canChangeSafety: Bool { connected && !armed }

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
