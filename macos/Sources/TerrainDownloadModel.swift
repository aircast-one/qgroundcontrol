import Foundation

struct TerrainDownload: Equatable {
    let loaded: Int
    let pending: Int

    static let none = TerrainDownload(loaded: 0, pending: 0)
    static let lingerSeconds = 30.0

    var total: Int { loaded + pending }

    var busy: Bool { pending > 0 }

    var started: Bool { loaded > 0 || pending > 0 }

    var fraction: Double {
        guard total > 0 else { return 0 }
        return Double(loaded) / Double(total)
    }

    var percentText: String { "\(Int((fraction * 100).rounded()))%" }

    var text: String {
        guard started else { return "" }
        return busy
            ? "Loading terrain \(loaded) of \(total)"
            : "Terrain loaded, \(loaded) block\(loaded == 1 ? "" : "s")"
    }

    static func showing(_ reading: TerrainDownload, sinceIdle: Double?) -> Bool {
        guard reading.started else { return false }
        if reading.busy { return true }
        guard let sinceIdle else { return false }
        return sinceIdle < lingerSeconds
    }

    static func read(_ facts: [[String: Any]]) -> TerrainDownload {
        func value(_ name: String) -> Int {
            facts.first { ($0["name"] as? String) == name }
                .flatMap { ($0["value"] as? NSNumber)?.intValue } ?? 0
        }
        return TerrainDownload(loaded: value("blocksLoaded"), pending: value("blocksPending"))
    }
}
