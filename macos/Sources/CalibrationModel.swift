import Foundation

struct CalibrationSide: Identifiable, Equatable {
    enum Stage: Equatable {
        case waiting
        case inProgress
        case done
    }

    let key: String
    let title: String
    let visible: Bool
    let stage: Stage
    let rotate: Bool

    var id: String { key }

    var symbol: String {
        switch stage {
        case .done: return "checkmark.circle.fill"
        case .inProgress: return rotate ? "arrow.triangle.2.circlepath" : "arrow.down.circle"
        case .waiting: return "circle"
        }
    }
}

struct CalibrationState: Equatable {
    var connected = false
    var inProgress = false
    var showsSides = false
    var nextEnabled = false
    var cancelEnabled = false
    var waitingForCancel = false
    var progress = 0.0
    var helpText = ""
    var statusText = ""
    var accelNeeded = false
    var compassNeeded = false
    var sides: [CalibrationSide] = []

    static let disconnected = CalibrationState()

    var visibleSides: [CalibrationSide] { sides.filter(\.visible) }

    var busy: Bool { inProgress || waitingForCancel }

    var progressText: String { String(format: "%.0f%%", progress) }

    var needsAttention: String {
        if accelNeeded && compassNeeded { return "The accelerometer and compass both need calibrating." }
        if accelNeeded { return "The accelerometer needs calibrating." }
        if compassNeeded { return "The compass needs calibrating." }
        return ""
    }
}

enum Calibration {
    static let sideOrder: [(key: String, title: String)] = [
        ("Down", "Level"),
        ("UpsideDown", "Upside down"),
        ("Left", "Left side"),
        ("Right", "Right side"),
        ("NoseDown", "Nose down"),
        ("TailDown", "Tail down"),
    ]

    static func property(_ key: String, _ suffix: String) -> String {
        "orientationCal\(key)Side\(suffix)"
    }

    static func sides(from json: [String: Any]) -> [CalibrationSide] {
        sideOrder.map { entry in
            func flag(_ suffix: String) -> Bool {
                (json[property(entry.key, suffix)] as? NSNumber)?.boolValue ?? false
            }
            let stage: CalibrationSide.Stage = flag("Done")
                ? .done
                : (flag("InProgress") ? .inProgress : .waiting)
            return CalibrationSide(key: entry.key, title: entry.title,
                                   visible: flag("Visible"), stage: stage, rotate: flag("Rotate"))
        }
    }

    static func read(_ json: [String: Any]) -> CalibrationState {
        guard json["kind"] as? String == "object" else { return .disconnected }

        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }

        var state = CalibrationState()
        state.connected = true
        state.inProgress = flag("calibrationInProgress")
        state.showsSides = flag("showOrientationCalArea")
        state.nextEnabled = flag("nextEnabled")
        state.cancelEnabled = flag("cancelEnabled")
        state.waitingForCancel = flag("waitingForCancel")
        state.progress = (json["calProgress"] as? NSNumber)?.doubleValue ?? 0
        state.helpText = (json["orientationHelpText"] as? String) ?? ""
        state.statusText = (json["statusText"] as? String) ?? ""
        state.accelNeeded = flag("accelSetupNeeded")
        state.compassNeeded = flag("compassSetupNeeded")
        state.sides = sides(from: json)
        return state
    }
}

enum CalibrationRoutine: String, CaseIterable, Identifiable {
    case accelerometer
    case compass
    case levelHorizon
    case gyro
    case pressure

    var id: String { rawValue }

    var needsAccelerometerFirst: Bool {
        switch self {
        case .compass, .levelHorizon: return true
        case .accelerometer, .gyro, .pressure: return false
        }
    }

    func blocked(whenAccelNeeded accelNeeded: Bool) -> Bool {
        needsAccelerometerFirst && accelNeeded
    }

    static let accelFirst = "Calibrate the accelerometer first."

    func description(whenAccelNeeded accelNeeded: Bool) -> String {
        blocked(whenAccelNeeded: accelNeeded) ? CalibrationRoutine.accelFirst : explanation
    }

    var title: String {
        switch self {
        case .accelerometer: return "Accelerometer"
        case .compass: return "Compass"
        case .levelHorizon: return "Level Horizon"
        case .gyro: return "Gyro"
        case .pressure: return "Pressure"
        }
    }

    var explanation: String {
        switch self {
        case .accelerometer: return "Hold the vehicle in each orientation it asks for."
        case .compass: return "Rotate the vehicle about every axis until each side is done."
        case .levelHorizon: return "Set the current attitude as level. Stand the vehicle flat first."
        case .gyro: return "Leave the vehicle still while the gyros settle."
        case .pressure: return "Zero the barometer at the current altitude."
        }
    }

    var invocation: String {
        switch self {
        case .accelerometer: return "sensorsCal.calibrateAccel"
        case .compass: return "sensorsCal.calibrateCompass"
        case .levelHorizon: return "sensorsCal.levelHorizon"
        case .gyro: return "sensorsCal.calibrateGyro"
        case .pressure: return "sensorsCal.calibratePressure"
        }
    }

    var arguments: [Any] { self == .accelerometer ? [false] : [] }
}
