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
