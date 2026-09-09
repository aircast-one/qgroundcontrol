import Foundation

// Split from NativeProbe so the pure-logic checks can see it: the registry needs AppKit
// for window and lock state, the protocol needs nothing.
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

// A window that only reads a store still wants the store visible to the probe, but must not
// hand out a second route to its actions: the plan store's own probe can upload a mission.
final class ReadOnlyProbe: Probeable {
    static let probeID = "readOnly"

    let identifier: String
    private let source: Probeable

    init(_ source: Probeable, as identifier: String) {
        self.source = source
        self.identifier = identifier
    }

    func probeState() -> [String: Any] { source.probeState() }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        ["ok": false, "error": "\(identifier) is read-only"]
    }
}
