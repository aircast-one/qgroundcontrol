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
    let units: String
    let scaleMaximum: Double
    let warningLevel: Double
    let dangerLevel: Double
    let axes: [VibrationAxis]
    let worst: Severity?
    let clipCounts: [Int]
    let clipping: Bool

    static let unavailable = VibrationReading()

    var emptyText: String {
        connected
            ? "This vehicle is not reporting vibration."
            : VehicleSetupText.connectPrompt(for: "vibration")
    }

    private init() {
        available = false
        connected = false
        units = ""
        scaleMaximum = 90
        warningLevel = 30
        dangerLevel = 60
        axes = []
        worst = nil
        clipCounts = []
        clipping = false
    }

    init(_ json: [String: Any], connected: Bool) {
        available = (json["available"] as? NSNumber)?.boolValue ?? false
        self.connected = connected
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
