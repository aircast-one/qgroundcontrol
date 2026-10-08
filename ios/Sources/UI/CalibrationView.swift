import Foundation

let CALIBRATION = "view.calibration"

struct CalibrationSide: Equatable {
    var key: String
    var title: String
    var visible: Bool
    var stage: String
    var rotate: Bool
}

struct CalibrationRoutine: Equatable {
    var id: String
    var title: String
    var invocation: String
    var arguments: [Bool]
    var blocked: Bool
    var enabled: Bool
    var description: String
    var dialogHelp: String
    var status: String
    var warning: String
    var spinsPropeller: Bool
    var dialogTitle: String = ""
}

struct CalibrationState: Equatable {
    var connected: Bool
    var inProgress: Bool
    var showsSides: Bool
    var nextEnabled: Bool
    var cancelEnabled: Bool
    var progress: Double
    var waitingForCancel: Bool = false
    var helpText: String
    var statusText: String
    var completed: String = ""
    var accelNeeded: Bool
    var compassNeeded: Bool
    var needsAttention: String
    var px4: Bool
    var settingsTitle: String
    var settingsDialogTitle: String = ""
    var fastCompass: FastCompass?
    var sides: [CalibrationSide]
    var routines: [CalibrationRoutine]
    var compassResults: [CompassResult] = []
}

struct CompassResult: Equatable {
    var compass: Int
    var green: Double
    var yellow: Double
    var range: Double
    var position: Double
}

private func list<T>(_ view: JSON, _ key: String, _ item: (JSON) -> T) -> [T] {
    (view[key].arrayOrNil ?? []).filter { $0.object != nil }.map(item)
}

func calibrationState(_ view: JSON?) -> CalibrationState? {
    guard let view, view["class"].string == "Calibration" else { return nil }
    return CalibrationState(
        connected: view["connected"].bool,
        inProgress: view["inProgress"].bool,
        showsSides: view["showsSides"].bool,
        nextEnabled: view["nextEnabled"].bool,
        cancelEnabled: view["cancelEnabled"].bool,
        progress: view["progress"].double(0),
        waitingForCancel: view["waitingForCancel"].bool,
        helpText: view["helpText"].string,
        statusText: view["statusText"].string,
        completed: view["completed"].string,
        accelNeeded: view["accelNeeded"].bool,
        compassNeeded: view["compassNeeded"].bool,
        needsAttention: view["needsAttention"].string,
        px4: view["px4"].bool,
        settingsTitle: view["settingsTitle"].string,
        settingsDialogTitle: view["settingsDialogTitle"].string,
        fastCompass: fastCompass(view["fastCompass"].object != nil ? view["fastCompass"] : nil),
        sides: list(view, "sides") {
            CalibrationSide(
                key: $0["key"].string,
                title: $0["title"].string,
                visible: $0["visible"].bool,
                stage: $0["stage"].string,
                rotate: $0["rotate"].bool
            )
        },
        routines: list(view, "routines") {
            CalibrationRoutine(
                id: $0["id"].string,
                title: $0["title"].string,
                invocation: $0["invocation"].string,
                arguments: $0["arguments"].array.map(\.bool),
                blocked: $0["blocked"].bool,
                enabled: $0["enabled"].bool,
                description: $0["description"].string,
                dialogHelp: $0["dialogHelp"].string,
                status: $0["status"].string,
                warning: $0["warning"].string,
                spinsPropeller: $0["spinsPropeller"].bool,
                dialogTitle: $0["dialogTitle"].string
            )
        },
        compassResults: list(view, "compassResults") {
            CompassResult(
                compass: $0["compass"].int(0),
                green: $0["green"].double(.nan),
                yellow: $0["yellow"].double(.nan),
                range: $0["range"].double(.nan),
                position: $0["position"].double(.nan)
            )
        }
    )
}

let CANCEL_WAIT_TITLE = "Cancelling calibration"
let CANCEL_WAIT_TEXT = "Waiting for Vehicle to response to Cancel. This may take a few seconds."

let COMPASS_ROUTINE = "compass"

let REBOOT_VEHICLE = "vehicle.rebootVehicle"

func runningTitle(_ name: String) -> String {
    let trimmed = name.trimmed
    return trimmed.isBlank ? "Calibration in progress" : "Calibrating \(trimmed)"
}

let SENSOR_HEALTH = "view.sensors"

struct SensorHealth: Equatable {
    var name: String
    var state: String
    var label: String
}

struct SensorHealthReading: Equatable {
    var available: Bool
    var sensors: [SensorHealth]
    var failing: [String]
    var status: String
}

func sensorHealth(_ view: JSON?) -> SensorHealthReading? {
    guard let view, view["class"].string == "SensorHealth" else { return nil }
    return SensorHealthReading(
        available: view["available"].bool,
        sensors: (view["sensors"].arrayOrNil ?? [])
            .filter { $0.object != nil && !$0["name"].string.isBlank }
            .map { SensorHealth(name: $0["name"].string, state: $0["state"].string, label: $0["label"].string) },
        failing: view["failing"].array.map(\.string).filter { !$0.isBlank },
        status: view["status"].string
    )
}

func healthSummary(_ reading: SensorHealthReading?) -> String {
    guard let reading, reading.available else { return "" }
    switch reading.failing.count {
    case 1: return "\(reading.failing[0]) is reporting a fault."
    case let count where count > 1: return "\(count) sensors are reporting faults."
    default: return ""
    }
}
