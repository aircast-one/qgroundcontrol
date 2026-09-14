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
    var gpsLockText = ""

    var batteryLevel = Level.unknown

    // A 3D fix is the point at which position is trustworthy enough to fly on; below
    // that the vehicle knows roughly where it is at best.
    var gpsLevel: Level {
        guard let gpsLock else { return .unknown }
        if gpsLock >= 3 { return .good }
        if gpsLock == 2 { return .warning }
        return .critical
    }

    // The lock's words are QGC's, not this head's. GPSFact.json carries the whole table --
    // translated, and it already names 7 "Static (fixed)", which is a BASE STATION rather than a
    // rover holding an RTK solution. A hand-written table here defaulted anything past RTK float
    // to "RTK fixed", so a base station and GPS_FIX_TYPE_PPP both read as the best fix there is:
    // the wrong answer was also the flattering one. The Fact arrives with its valueString beside
    // its number and this now spells nothing itself.
    var gpsText: String {
        guard gpsLock != nil else { return "—" }
        let fix = gpsLockText
        guard let satellites else { return fix.isEmpty ? "—" : fix }
        return fix.isEmpty ? "\(satellites) sats" : "\(fix) · \(satellites) sats"
    }

    var batteryText = "—"

    static func batteryLine(_ main: String, _ secondary: String) -> String {
        guard !main.isEmpty else { return "—" }
        guard !secondary.isEmpty, secondary != main else { return main }
        return "\(main) · \(secondary)"
    }

    // The core composes a headline for EVERY pack, not just the first. The head used packs[0]'s
    // and gave rows 2+ whatever their first detail fact happened to be, so a second battery's
    // summary was a different quantity in a different shape from the one above it.
    static func batteryHeadlines(_ packs: [[String: Any]]) -> [String] {
        packs.map {
            batteryLine(($0["text"] as? String) ?? "", ($0["secondaryText"] as? String) ?? "")
        }
    }



}
