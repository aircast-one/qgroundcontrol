import Foundation

enum PreflightAirframe: String, Equatable {
    case multiRotor = "Multirotor"
    case vtol = "VTOL"
    case rover = "Rover"
    case sub = "Submarine"
    case fixedWing = "Fixed wing"
    case generic = "Generic"

    static func of(multiRotor: Bool, vtol: Bool, rover: Bool, sub: Bool, fixedWing: Bool) -> PreflightAirframe {
        multiRotor ? .multiRotor
            : vtol ? .vtol
            : rover ? .rover
            : sub ? .sub
            : fixedWing ? .fixedWing
            : .generic
    }

    var hardwarePrompt: String {
        switch self {
        case .multiRotor: return "Props mounted and secured?"
        case .rover: return "Battery mounted and secured?"
        case .sub: return "All seals in place?"
        case .vtol, .fixedWing, .generic: return "Props mounted? Wings secured? Tail secured?"
        }
    }

    var checksActuators: Bool {
        switch self {
        case .multiRotor, .rover: return false
        case .vtol, .sub, .fixedWing, .generic: return true
        }
    }

    var checksMotors: Bool { self != .rover }

    var windPrompt: String? {
        switch self {
        case .sub: return nil
        case .multiRotor, .rover: return "Within limits for this airframe?"
        case .vtol, .fixedWing, .generic: return "Within limits, and are you launching into the wind?"
        }
    }

    var area: (name: String, prompt: String)? {
        switch self {
        case .sub: return nil
        case .rover: return ("Mission area", "Mission area and path clear of obstacles and people?")
        case .multiRotor, .vtol, .fixedWing, .generic:
            return ("Flight area", "Launch area and path clear of obstacles and people?")
        }
    }
}

struct PreflightCheck: Identifiable, Equatable {
    enum Verdict: Equatable {
        case manual
        case passing
        case failing(String)
        case overridable(String)
    }

    let name: String
    let prompt: String
    let verdict: Verdict

    var id: String { name }

    var blocked: Bool {
        if case .failing = verdict { return true }
        return false
    }

    var reason: String {
        switch verdict {
        case .manual: return prompt
        case .passing: return prompt
        case .failing(let why), .overridable(let why): return why
        }
    }
}

struct PreflightGroup: Identifiable, Equatable {
    let name: String
    let checks: [PreflightCheck]

    var id: String { name }
}

enum Preflight {
    static let failureSatCount = 9
    static let failurePercent = 40.0
    static let sensorMask = 1 | 2 | 4 | 8 | 16 | 32 | 2097152

    static func gps(lock: Int?, satellites: Int?) -> PreflightCheck {
        let prompt = "3D lock and enough satellites."
        guard let lock else {
            return PreflightCheck(name: "GPS", prompt: prompt,
                                  verdict: .failing("No vehicle is reporting a GPS."))
        }
        guard lock >= 3 else {
            return PreflightCheck(name: "GPS", prompt: prompt, verdict: .failing("Waiting for 3D lock."))
        }
        let count = satellites ?? 0
        guard count >= failureSatCount else {
            return PreflightCheck(
                name: "GPS", prompt: prompt,
                verdict: .overridable("Only \(count) satellite\(count == 1 ? "" : "s"); \(failureSatCount) wanted."))
        }
        return PreflightCheck(name: "GPS", prompt: prompt, verdict: .passing)
    }

    static func battery(percent: Double?) -> PreflightCheck {
        let prompt = "Battery connector firmly plugged?"
        guard let percent else {
            return PreflightCheck(name: "Battery", prompt: prompt,
                                  verdict: .failing("No vehicle is reporting a battery."))
        }
        guard percent >= failurePercent else {
            return PreflightCheck(
                name: "Battery", prompt: prompt,
                verdict: .failing(String(format: "Charge is %.0f%%, below %.0f%%. Recharge.",
                                         percent, failurePercent)))
        }
        return PreflightCheck(name: "Battery", prompt: prompt, verdict: .passing)
    }

    static func sensors(unhealthyBits: Int?) -> PreflightCheck {
        let prompt = "Every sensor the autopilot needs is healthy."
        guard let unhealthyBits else {
            return PreflightCheck(name: "Sensors", prompt: prompt,
                                  verdict: .failing("No vehicle is reporting sensor health."))
        }
        let failing = unhealthyBits & sensorMask
        guard failing == 0 else {
            return PreflightCheck(name: "Sensors", prompt: prompt,
                                  verdict: .failing(names(of: failing) + " unhealthy."))
        }
        return PreflightCheck(name: "Sensors", prompt: prompt, verdict: .passing)
    }

    static func names(of bits: Int) -> String {
        let known = [(1, "Gyro"), (2, "Accelerometer"), (4, "Magnetometer"),
                     (8, "Barometer"), (16, "Airspeed"), (32, "GPS"), (2097152, "AHRS")]
        let listed = known.filter { bits & $0.0 != 0 }.map(\.1)
        return listed.isEmpty ? "A sensor is" : listed.joined(separator: ", ")
    }

    static func manual(_ name: String, _ prompt: String) -> PreflightCheck {
        PreflightCheck(name: name, prompt: prompt, verdict: .manual)
    }

    static func sound(muted: Bool) -> PreflightCheck {
        let prompt = "QGC audio warnings are on. Is the system output on too?"
        guard muted else {
            return PreflightCheck(name: "Sound output", prompt: prompt, verdict: .passing)
        }
        return PreflightCheck(
            name: "Sound output", prompt: prompt,
            verdict: .failing("QGC audio output is muted; enable it in Settings to hear warnings."))
    }

    static func groups(airframe: PreflightAirframe, lock: Int?, satellites: Int?,
                       batteryPercent: Double?, unhealthyBits: Int?,
                       audioMuted: Bool) -> [PreflightGroup] {
        [
            PreflightGroup(name: "Before you power up", checks: [
                manual("Hardware", airframe.hardwarePrompt),
                battery(percent: batteryPercent),
                sensors(unhealthyBits: unhealthyBits),
                gps(lock: lock, satellites: satellites),
                manual("Radio control", "Receiving signal. Range test done and confirmed?"),
            ]),
            PreflightGroup(name: "Arm the vehicle here", checks: [
                airframe.checksActuators
                    ? manual("Actuators", "Move every control surface. Did they all work properly?") : nil,
                airframe.checksMotors
                    ? manual("Motors", "Propellers free? Throttle up gently — working properly?") : nil,
                manual("Mission", "Waypoints valid and no terrain collision?"),
                sound(muted: audioMuted),
            ].compactMap { $0 }),
            PreflightGroup(name: "Before launch", checks: [
                manual("Payload", "Configured, started, and the lid closed?"),
                airframe.windPrompt.map { manual("Wind and weather", $0) },
                airframe.area.map { manual($0.name, $0.prompt) },
            ].compactMap { $0 }),
        ]
    }

    static func total(_ groups: [PreflightGroup]) -> Int {
        groups.reduce(0) { $0 + $1.checks.count }
    }

    static func progress(_ groups: [PreflightGroup], ticked: Set<String>) -> String {
        let present = Set(groups.flatMap(\.checks).map(\.name))
        return "\(ticked.intersection(present).count) of \(total(groups)) checked"
    }

    static func ready(_ groups: [PreflightGroup], ticked: Set<String>) -> Bool {
        groups.allSatisfy { group in
            group.checks.allSatisfy { ticked.contains($0.name) }
        }
    }
}
