import Foundation
import QGCBridgeC

enum Bridge {
    struct Reading {
        let text: String
        let units: String

        static let absent = Reading(text: "—", units: "")
    }

    enum Stats {
        private(set) static var calls = 0
        private(set) static var lastMillis = 0.0
        private(set) static var worstMillis = 0.0
        private(set) static var totalMillis = 0.0

        static func record(_ millis: Double) {
            calls += 1
            lastMillis = millis
            worstMillis = max(worstMillis, millis)
            totalMillis += millis
        }

        static func reset() {
            calls = 0
            lastMillis = 0
            worstMillis = 0
            totalMillis = 0
        }

        static var snapshot: [String: Any] {
            [
                "calls": calls,
                "lastMillis": lastMillis,
                "worstMillis": worstMillis,
                "meanMillis": calls > 0 ? totalMillis / Double(calls) : 0,
            ]
        }
    }

    private static func json(_ path: String) -> [String: Any] {
        let started = DispatchTime.now().uptimeNanoseconds
        defer { Stats.record(Double(DispatchTime.now().uptimeNanoseconds - started) / 1_000_000) }
        guard let raw = qgc_bridge_get(path) else { return [:] }
        defer { qgc_bridge_free(raw) }
        return decode(raw)
    }

    static func read(_ path: String) -> Reading {
        format(json(path))
    }

    static func watch(_ paths: [String]) {
        qgc_bridge_watch(paths.joined(separator: ","))
    }

    static func onEvent(_ handler: @escaping (String, [String: Any]) -> Void) {
        eventHandler = handler
        qgc_bridge_set_event_handler(bridgeEventTrampoline)
    }

    fileprivate static var eventHandler: ((String, [String: Any]) -> Void)?

    fileprivate static func decode(_ raw: UnsafePointer<CChar>) -> [String: Any] {
        let data = Data(bytes: raw, count: strlen(raw))
        return (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
    }

    private static func format(_ json: [String: Any]) -> Reading {
        switch json["kind"] as? String {
        case "fact":
            let units = json["units"] as? String ?? ""
            let text = json["enumOrValueString"] as? String
                ?? json["valueString"] as? String
                ?? describe(json["value"])
            return Reading(text: text, units: units)
        case "value":
            return Reading(text: describe(json["value"]), units: "")
        case "object":
            return Reading(text: "<object>", units: "")
        default:
            return .absent
        }
    }

    static func bool(_ path: String) -> Bool {
        (json(path)["value"] as? NSNumber)?.boolValue ?? false
    }

    private static func describe(_ value: Any?) -> String {
        guard let value else { return "—" }
        if let number = value as? NSNumber {
            if CFGetTypeID(number) == CFBooleanGetTypeID() {
                return number.boolValue ? "yes" : "no"
            }
            let double = number.doubleValue
            return double == double.rounded() ? String(number.intValue) : String(format: "%.2f", double)
        }
        if let string = value as? String {
            return string.isEmpty ? "—" : string
        }
        return "—"
    }
}

private let bridgeEventTrampoline: QGCBridgeEventFn = { path, json in
    guard let path, let json else { return }
    let name = String(cString: path)
    let decoded = Bridge.decode(json)
    DispatchQueue.main.async { Bridge.eventHandler?(name, decoded) }
}
