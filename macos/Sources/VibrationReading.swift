import Foundation

struct VibrationReading {
    // ArduPilot and PX4 guidance: sustained vibration above ~30 degrades attitude
    // estimation and above ~60 is likely to cause a crash. The QML page draws its
    // threshold lines at exactly these values; they are the reason the page exists.
    static let scaleMaximum = 90.0
    static let warningLevel = 30.0
    static let dangerLevel = 60.0

    let x: Double
    let y: Double
    let z: Double
    let clipCounts: [Int]
    let available: Bool

    static let unavailable = VibrationReading(x: 0, y: 0, z: 0, clipCounts: [0, 0, 0], available: false)

    enum Severity: Equatable { case normal, warning, danger }

    static func severity(_ value: Double) -> Severity {
        if value >= dangerLevel { return .danger }
        if value >= warningLevel { return .warning }
        return .normal
    }

    var worst: Severity { VibrationReading.severity(max(x, max(y, z))) }
}

