import Foundation

struct FlyTelemetry: Equatable {
    enum Level {
        case good
        case caution
        case warning
        case critical
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "normal": self = .good
            case "caution": self = .caution
            case "warning": self = .warning
            case "critical": self = .critical
            default: self = .unknown
            }
        }
    }

    var heading: Double?
    var satellites: Int?
    var gpsLock: Int?

    var batteryLevel = Level.unknown

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

    var batteryText = "—"

    static func batteryLine(_ main: String, _ secondary: String) -> String {
        guard !main.isEmpty else { return "—" }
        guard !secondary.isEmpty, secondary != main else { return main }
        return "\(main) · \(secondary)"
    }



}
