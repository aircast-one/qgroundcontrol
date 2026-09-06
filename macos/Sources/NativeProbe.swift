import AppKit
import Foundation

// Driving a locked screen through synthesised input does not work: no window can become
// key, so clicks either go to the wrong place or are swallowed, and both look like a
// pass. Screenshots by window id keep working. So the supported way to drive the native
// UI in tests is by identity — read state, invoke a named action — which is the direct
// counterpart of QGC's objectName-addressed /ui/* surface for QML.
protocol Probeable: AnyObject {
    static var probeID: String { get }
    func probeState() -> [String: Any]
    func probeInvoke(action: String, args: [String: String]) -> [String: Any]
}

extension Probeable {
    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        ["ok": false, "error": "\(Self.probeID) has no actions"]
    }
}

enum NativeProbe {
    private static var registry: [String: Probeable] = [:]

    static func register(_ target: Probeable) {
        registry[type(of: target).probeID] = target
    }

    static var screenIsLocked: Bool {
        guard let session = CGSessionCopyCurrentDictionary() as? [String: Any] else { return false }
        return (session["CGSSessionScreenIsLocked"] as? NSNumber)?.boolValue ?? false
    }

    static func tree() -> [String: Any] {
        [
            "screenIsLocked": screenIsLocked,
            "probes": registry.keys.sorted(),
            "state": registry.mapValues { $0.probeState() },
        ]
    }

    static func state(of id: String) -> [String: Any] {
        guard let target = registry[id] else {
            return ["ok": false, "error": "no probe \(id)", "probes": registry.keys.sorted()]
        }
        return ["ok": true, "state": target.probeState()]
    }

    static func invoke(id: String, action: String, args: [String: String]) -> [String: Any] {
        guard let target = registry[id] else {
            return ["ok": false, "error": "no probe \(id)", "probes": registry.keys.sorted()]
        }
        return target.probeInvoke(action: action, args: args)
    }
}
