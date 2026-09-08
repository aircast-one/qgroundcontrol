import Foundation
import QGCBridgeC

enum Bridge {
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

    @discardableResult
    static func set(_ path: String, _ value: Any) -> Bool {
        let payload = (try? JSONSerialization.data(withJSONObject: ["value": value]))
            .flatMap { String(data: $0, encoding: .utf8) } ?? "{}"
        guard let raw = qgc_bridge_set(path, payload) else { return false }
        defer { qgc_bridge_free(raw) }
        return decode(raw)["ok"] as? Bool ?? false
    }

    @discardableResult
    static func invoke(_ path: String, _ args: [Any] = []) -> [String: Any] {
        let argsJson = (try? JSONSerialization.data(withJSONObject: args))
            .flatMap { String(data: $0, encoding: .utf8) } ?? "[]"
        guard let raw = qgc_bridge_invoke(path, argsJson) else { return [:] }
        defer { qgc_bridge_free(raw) }
        return decode(raw)
    }

    static func group(_ path: String) -> [String: Any] {
        json(path)
    }

    private static func decode(_ raw: UnsafePointer<CChar>) -> [String: Any] {
        let data = Data(bytes: raw, count: strlen(raw))
        return (try? JSONSerialization.jsonObject(with: data)) as? [String: Any] ?? [:]
    }
}

protocol WriteReporting: AnyObject {
    var writeFailure: String? { get set }
}

extension WriteReporting {
    @discardableResult
    func write(_ path: String, _ value: Any, _ what: String) -> Bool {
        guard Bridge.set(path, value) else {
            writeFailure = WriteReport.failure(what)
            return false
        }
        return true
    }
}
