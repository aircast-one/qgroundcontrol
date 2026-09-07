import Foundation

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

    static func groups(lock: Int?, satellites: Int?, batteryPercent: Double?,
                       unhealthyBits: Int?) -> [PreflightGroup] {
        [
            PreflightGroup(name: "Before you power up", checks: [
                manual("Hardware", "Props mounted and secured?"),
                battery(percent: batteryPercent),
                sensors(unhealthyBits: unhealthyBits),
                gps(lock: lock, satellites: satellites),
            ]),
            PreflightGroup(name: "Arm the vehicle here", checks: [
                manual("Motors", "Propellers free? Throttle up gently — working properly?"),
                manual("Mission", "Waypoints valid and no terrain collision?"),
            ]),
            PreflightGroup(name: "Before launch", checks: [
                manual("Payload", "Configured, started, and the lid closed?"),
                manual("Wind and weather", "Within limits for this airframe?"),
                manual("Flight area", "Launch area and path clear of obstacles and people?"),
            ]),
        ]
    }

    static func total(_ groups: [PreflightGroup]) -> Int {
        groups.reduce(0) { $0 + $1.checks.count }
    }

    static func progress(_ groups: [PreflightGroup], ticked: Set<String>) -> String {
        "\(ticked.count) of \(total(groups)) checked"
    }

    static func ready(_ groups: [PreflightGroup], ticked: Set<String>) -> Bool {
        groups.allSatisfy { group in
            group.checks.allSatisfy { ticked.contains($0.name) }
        }
    }
}
