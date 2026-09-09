import Foundation

struct FlightModeChoice: Identifiable, Equatable {
    let name: String
    let summary: String
    let advanced: Bool
    let current: Bool
    let needsConfirm: Bool

    var id: String { name }
    var symbol: String { FlightModes.symbol(for: name) }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        summary = (json["summary"] as? String) ?? ""
        advanced = (json["advanced"] as? NSNumber)?.boolValue ?? false
        current = (json["current"] as? NSNumber)?.boolValue ?? false
        needsConfirm = (json["needsConfirm"] as? NSNumber)?.boolValue ?? false
    }
}

enum FlightModes {
    static let glyphs: [(keywords: [String], symbol: String)] = [
        (["rtl", "return"], "house"),
        (["land", "dock"], "arrow.down.to.line"),
        (["takeoff"], "arrow.up.to.line"),
        (["auto", "mission"], "list.bullet"),
        (["guided", "offboard"], "hand.tap"),
        (["loiter", "circle", "orbit", "hold", "brake", "position"], "circle.dashed"),
        (["follow"], "figure.walk"),
        (["acro", "sport", "flip", "rattitude", "turtle"], "arrow.triangle.2.circlepath"),
        (["altitude", "depth", "surface", "surftrak"], "arrow.up.arrow.down"),
        (["autotune", "systemid", "motor detection"], "slider.horizontal.3"),
        (["stabilize", "stabilized", "manual", "training", "steering"], "hand.raised"),
    ]

    static func symbol(for mode: String) -> String {
        let name = mode.lowercased()
        let match = glyphs.first { $0.keywords.contains { name.contains($0) } }
        return match?.symbol ?? "airplane"
    }

    static func list(_ json: Any?) -> [FlightModeChoice] {
        ((json as? [Any]) ?? []).compactMap(FlightModeChoice.init)
    }

    static func everyday(_ choices: [FlightModeChoice]) -> [FlightModeChoice] {
        choices.filter { !$0.advanced || $0.current }
    }

    static func folded(_ choices: [FlightModeChoice]) -> [FlightModeChoice] {
        choices.filter { $0.advanced && !$0.current }
    }
}
