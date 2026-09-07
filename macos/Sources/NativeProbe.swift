import AppKit
import Foundation

enum NativeProbe {
    private static var registry: [String: Probeable] = [:]

    static func register(_ target: Probeable) {
        registry[type(of: target).probeID] = target
    }

    static func register(_ target: Probeable, as id: String) {
        registry[id] = target
    }

    static var screenIsLocked: Bool {
        guard let session = CGSessionCopyCurrentDictionary() as? [String: Any] else { return false }
        return (session["CGSSessionScreenIsLocked"] as? NSNumber)?.boolValue ?? false
    }

    private static func onMain<T>(_ body: () -> T) -> T {
        Thread.isMainThread ? body() : DispatchQueue.main.sync(execute: body)
    }

    static func tree() -> [String: Any] {
        onMain {
            [
                "screenIsLocked": screenIsLocked,
                "probes": registry.keys.sorted(),
                "state": registry.mapValues { $0.probeState() },
            ]
        }
    }

    static func state(of id: String) -> [String: Any] {
        onMain {
            guard let target = registry[id] else {
                return ["ok": false, "error": "no probe \(id)", "probes": registry.keys.sorted()]
            }
            return ["ok": true, "state": target.probeState()]
        }
    }

    static func invoke(id: String, action: String, args: [String: String]) -> [String: Any] {
        onMain {
            guard let target = registry[id] else {
                return ["ok": false, "error": "no probe \(id)", "probes": registry.keys.sorted()]
            }
            return target.probeInvoke(action: action, args: args)
        }
    }
}
