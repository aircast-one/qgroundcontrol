import Foundation

struct VibrationAxis: Identifiable, Equatable {
    let axis: String
    let label: String
    let value: Double?
    let fraction: Double?
    let severity: VibrationReading.Severity?

    var id: String { axis }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let axis = json["axis"] as? String, !axis.isEmpty else { return nil }
        self.axis = axis
        label = (json["label"] as? String) ?? axis.uppercased()
        value = (json["value"] as? NSNumber)?.doubleValue
        fraction = (json["fraction"] as? NSNumber)?.doubleValue
        severity = VibrationReading.Severity(json["severity"] as? String)
    }
}

struct VibrationReading: Equatable {
    enum Severity {
        case normal
        case warning
        case danger

        init?(_ reported: String?) {
            switch reported {
            case "normal": self = .normal
            case "warning": self = .warning
            case "danger": self = .danger
            default: return nil
            }
        }
    }

    let available: Bool
    let connected: Bool
    let silentReason: String
    let units: String
    let scaleMaximum: Double
    let warningLevel: Double
    let dangerLevel: Double
    let axes: [VibrationAxis]
    let worst: Severity?
    let clipCounts: [Int]
    let clipping: Bool

    static let unavailable = VibrationReading()

    // Keyed on the core's TOKEN rather than on connected, which was this head inferring what the
    // silence meant from a neighbouring fact. The token IS the meaning, and it travels in the same
    // read as the axes it qualifies. The sentences stay this head's because this screen shows one
    // string and it should be an instruction -- silentText states what is true, which is the right
    // half for a screen that has room for both and the wrong half for a screen that has room for
    // one. An unrecognised token falls back to the prompt that still asks the operator to look.
    // A null silentReason does NOT mean the core had nothing to say: vibration.rs sets it only when
    // NO axis has a value, so null means at least one does. This screen needs all three to draw its
    // three bars, so a vehicle reporting two of them lands in the empty state with no reason token
    // -- and the old rule read that as noVehicle and said "Connect a vehicle", to somebody looking
    // at a connected one. The core said what the vehicle did; what this screen cannot draw is the
    // head's own business and the head has to say it.
    var emptyText: String {
        switch silentReason {
        case "notReported": return "This vehicle is not reporting vibration."
        case "": return connected
            ? "This vehicle is reporting only some vibration axes."
            : VehicleSetupText.connectPrompt(for: "vibration")
        default: return VehicleSetupText.connectPrompt(for: "vibration")
        }
    }

    private init() {
        available = false
        connected = false
        silentReason = ""
        units = ""
        scaleMaximum = 90
        warningLevel = 30
        dangerLevel = 60
        axes = []
        worst = nil
        clipCounts = []
        clipping = false
    }

    // view.vibration answers `connected` and the axes in ONE read. The store used to fetch
    // vehicles.activeVehicleAvailable separately, so a vehicle dropping between the two calls
    // produced a reading that was connected with no axes, or disconnected with axes. The view
    // already DEPENDED on that path to know when to recompute and then dropped the answer, which
    // is why the only way to get it was the second call that is the race.
    init(view json: [String: Any]) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        connected = (json["connected"] as? NSNumber)?.boolValue ?? false
        silentReason = (json["silentReason"] as? String) ?? ""
        units = (json["units"] as? String) ?? ""
        scaleMaximum = (json["scaleMaximum"] as? NSNumber)?.doubleValue ?? 90
        warningLevel = (json["warningLevel"] as? NSNumber)?.doubleValue ?? 30
        dangerLevel = (json["dangerLevel"] as? NSNumber)?.doubleValue ?? 60
        axes = ((json["axes"] as? [Any]) ?? []).compactMap(VibrationAxis.init)
        worst = Severity(json["worst"] as? String)
        clipCounts = ((json["clipCounts"] as? [Any]) ?? []).compactMap { ($0 as? NSNumber)?.intValue }
        clipping = (json["clipping"] as? NSNumber)?.boolValue ?? false
    }
}
