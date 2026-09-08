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
}
