import Foundation

struct FlyTelemetry: Equatable {
    enum Level {
        case good
        case warning
        case critical
        case unknown
    }

    var mode = ""
    var armed = false
    var flying = false
    var altitude: Double?
    var groundSpeed: Double?
    var climbRate: Double?
    var heading: Double?
    var batteryPercent: Double?
    var batteryVolts: Double?
    var satellites: Int?
    var gpsLock: Int?

    static let lowBattery = 25.0
    static let criticalBattery = 15.0

    var batteryLevel: Level {
        guard let batteryPercent else { return .unknown }
        if batteryPercent <= FlyTelemetry.criticalBattery { return .critical }
        if batteryPercent <= FlyTelemetry.lowBattery { return .warning }
        return .good
    }

    // A 3D fix is the point at which position is trustworthy enough to fly on; below
    // that the vehicle knows roughly where it is at best.
    var gpsLevel: Level {
        guard let gpsLock else { return .unknown }
        if gpsLock >= 3 { return .good }
        if gpsLock == 2 { return .warning }
        return .critical
    }

    var gpsText: String {
        guard let gpsLock else { return "—" }
        let fix: String
        switch gpsLock {
        case 0, 1: fix = "No fix"
        case 2: fix = "2D"
        case 3: fix = "3D"
        case 4: fix = "DGPS"
        case 5: fix = "RTK float"
        default: fix = "RTK fixed"
        }
        guard let satellites else { return fix }
        return "\(fix) · \(satellites) sats"
    }

    var batteryText: String {
        guard let batteryPercent else { return "—" }
        guard let batteryVolts else { return String(format: "%.0f%%", batteryPercent) }
        return String(format: "%.0f%% · %.1f V", batteryPercent, batteryVolts)
    }

    var stateText: String {
        if !armed { return "Disarmed" }
        return flying ? "Flying" : "Armed"
    }

    // A vehicle sitting on the ground reports a relative altitude a hair below zero,
    // like -0.04, which rounds to -0.0 m at one decimal place.
    private static func zeroed(_ value: Double) -> Double {
        let rounded = (value * 10).rounded() / 10
        return rounded == 0 ? 0 : rounded
    }

    static func metres(_ value: Double?) -> String {
        guard let value, value.isFinite else { return "—" }
        return String(format: "%.1f m", zeroed(value))
    }

    static func speed(_ value: Double?) -> String {
        guard let value, value.isFinite else { return "—" }
        return String(format: "%.1f m/s", zeroed(value))
    }

    static func degrees(_ value: Double?) -> String {
        guard let value, value.isFinite else { return "—" }
        return String(format: "%.0f°", value)
    }
}
