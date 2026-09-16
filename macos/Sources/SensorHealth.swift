import Foundation

struct SensorHealth: Identifiable, Equatable {
    enum State {
        case healthy
        case unhealthy
        case disabled
        case unknown

        init(_ reported: String?) {
            switch reported {
            case "healthy": self = .healthy
            case "unhealthy": self = .unhealthy
            case "disabled": self = .disabled
            default: self = .unknown
            }
        }
    }

    let name: String
    let state: State
    let label: String

    var id: String { name }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty,
              let reported = json["state"] as? String else { return nil }
        self.name = name
        state = State(reported)
        label = (json["label"] as? String) ?? ""
    }

    static func list(_ json: Any?) -> [SensorHealth] {
        ((json as? [Any]) ?? []).compactMap(SensorHealth.init)
    }

    // The summary row's sentence, its plural and its symbol lived in VehicleSetupWindow, which
    // swift-checks does not compile, so all three were two-armed ternaries nothing could fail on.
    // The fault arm is also the one this rig cannot produce -- the copter fake reports a single
    // healthy GPS -- so a screenshot would not have caught a swapped ternary either. `failing` is
    // the core's own list of unhealthy names (sensors.rs), not a count re-derived here.
    static func headline(_ failing: [String]) -> String {
        guard !failing.isEmpty else { return "All enabled sensors are healthy" }
        return "\(failing.count) sensor\(failing.count == 1 ? "" : "s") reporting a fault"
    }

    static func faultNames(_ failing: [String]) -> String {
        failing.joined(separator: ", ")
    }

    static func symbol(_ failing: [String]) -> String {
        failing.isEmpty ? "checkmark.circle.fill" : "exclamationmark.triangle.fill"
    }
}
