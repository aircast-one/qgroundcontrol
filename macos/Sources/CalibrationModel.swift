import Foundation

struct CalibrationSide: Identifiable, Equatable {
    enum Stage: Equatable {
        case waiting
        case inProgress
        case done
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "waiting": self = .waiting
            case "inProgress": self = .inProgress
            case "done": self = .done
            default: self = .unknown
            }
        }
    }

    let key: String
    let title: String
    let stage: Stage
    let rotate: Bool

    var id: String { key }

    var symbol: String {
        switch stage {
        case .done: return "checkmark.circle.fill"
        case .inProgress: return rotate ? "arrow.triangle.2.circlepath" : "arrow.down.circle"
        case .waiting, .unknown: return "circle"
        }
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let key = json["key"] as? String else { return nil }
        self.key = key
        title = (json["title"] as? String) ?? ""
        stage = Stage(json["stage"] as? String)
        rotate = (json["rotate"] as? NSNumber)?.boolValue ?? false
    }
}

struct CalibrationRoutine: Identifiable, Equatable {
    let id: String
    let title: String
    let invocation: String
    let arguments: [Bool]
    let blocked: Bool
    let enabled: Bool
    let description: String
    let warning: String
    // calibration.rs carries this per routine and serves it as "spinsPropeller". This head read
    // neither it nor the separate warning: the window CONCATENATED warning onto description, so
    // "This spins the motors." arrived as the tail of a six-line paragraph, under a Start button
    // identical to the five beside it -- one of which is "Leave the vehicle still while the gyros
    // settle". The core split the two fields so they could be drawn apart; joining them undid that.
    let spinsPropeller: Bool

    static let untruncated = 12

    // ONE WEIGHT FOR ONE FACT, and here the fact is that pressing this turns the props. The
    // warning TEXT is a display decision keyed on presence; the SEVERITY is keyed on the flag,
    // because a routine could gain a warning that is not about propellers and must not inherit
    // this weight.
    var severity: FlyTelemetry.Level { spinsPropeller ? .warning : .good }

    var descriptionLines: Int { warning.isEmpty ? 1 : CalibrationRoutine.untruncated }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let id = json["id"] as? String,
              let invocation = json["invocation"] as? String, !invocation.isEmpty else { return nil }
        self.id = id
        self.invocation = invocation
        title = (json["title"] as? String) ?? ""
        arguments = ((json["arguments"] as? [Any]) ?? [])
            .compactMap { ($0 as? NSNumber)?.boolValue }
        blocked = (json["blocked"] as? NSNumber)?.boolValue ?? false
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? false
        description = (json["description"] as? String) ?? ""
        warning = (json["warning"] as? String) ?? ""
        spinsPropeller = (json["spinsPropeller"] as? NSNumber)?.boolValue ?? false
    }
}

struct CalibrationState: Equatable {
    let connected: Bool
    let inProgress: Bool
    let busy: Bool
    let showsSides: Bool
    let nextEnabled: Bool
    let cancelEnabled: Bool
    let progress: Double
    let progressText: String
    let helpText: String
    let statusText: String
    let needsAttention: String
    let visibleSides: [CalibrationSide]
    let routines: [CalibrationRoutine]

    static let disconnected = CalibrationState()

    private init() {
        connected = false
        inProgress = false
        busy = false
        showsSides = false
        nextEnabled = false
        cancelEnabled = false
        progress = 0
        progressText = ""
        helpText = ""
        statusText = ""
        needsAttention = ""
        visibleSides = []
        routines = []
    }

    init(_ json: [String: Any]) {
        func flag(_ name: String) -> Bool { (json[name] as? NSNumber)?.boolValue ?? false }
        func text(_ name: String) -> String { (json[name] as? String) ?? "" }
        connected = flag("connected")
        inProgress = flag("inProgress")
        busy = flag("busy")
        showsSides = flag("showsSides")
        nextEnabled = flag("nextEnabled")
        cancelEnabled = flag("cancelEnabled")
        progress = (json["progress"] as? NSNumber)?.doubleValue ?? 0
        progressText = text("progressText")
        helpText = text("helpText")
        statusText = text("statusText")
        needsAttention = text("needsAttention")
        visibleSides = ((json["visibleSides"] as? [Any]) ?? []).compactMap(CalibrationSide.init)
        routines = ((json["routines"] as? [Any]) ?? []).compactMap(CalibrationRoutine.init)
    }
}
