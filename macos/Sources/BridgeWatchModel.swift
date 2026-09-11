import Foundation

struct BridgeWatchers {
    private(set) var watched: [String: [String]] = [:]

    static let separator = ","

    mutating func set(_ client: String, _ paths: [String]) {
        watched[client] = paths.isEmpty ? nil : paths
    }

    func clients(watching path: String) -> [String] {
        watched.filter { $0.value.contains(path) }.keys.sorted()
    }

    func csv(_ client: String) -> String {
        (watched[client] ?? []).joined(separator: BridgeWatchers.separator)
    }

}
