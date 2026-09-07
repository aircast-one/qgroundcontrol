import Foundation

struct SensorHealth: Identifiable {
    enum State {
        case healthy
        case unhealthy
        case disabled
    }

    let name: String
    let state: State
    var id: String { name }

    // A disabled sensor also reports unhealthy, which would put Geofence and Logging
    // in the same bucket as a failed GPS. Not enabled is not the same as broken.
    init(name: String, enabled: Bool, healthy: Bool) {
        self.name = name
        if !enabled {
            state = .disabled
        } else {
            state = healthy ? .healthy : .unhealthy
        }
    }

    static func from(json: [String: Any]) -> [SensorHealth] {
        let names = (json["sensorNames"] as? [String]) ?? []
        let enabled = (json["sensorEnabled"] as? [Any]) ?? []
        let healthy = (json["sensorHealthy"] as? [Any]) ?? []
        guard names.count == enabled.count, names.count == healthy.count else { return [] }

        return names.indices.map { index in
            SensorHealth(name: names[index],
                         enabled: (enabled[index] as? NSNumber)?.boolValue ?? false,
                         healthy: (healthy[index] as? NSNumber)?.boolValue ?? false)
        }
    }

    // Anything failing comes first: the operator is looking for what is wrong, and
    // scanning seventeen green rows to find one red one is the wrong way round.
    static func ordered(_ sensors: [SensorHealth]) -> [SensorHealth] {
        let rank: (State) -> Int = { state in
            switch state {
            case .unhealthy: return 0
            case .healthy: return 1
            case .disabled: return 2
            }
        }
        return sensors.enumerated().sorted {
            rank($0.element.state) == rank($1.element.state)
                ? $0.offset < $1.offset
                : rank($0.element.state) < rank($1.element.state)
        }.map(\.element)
    }
}
