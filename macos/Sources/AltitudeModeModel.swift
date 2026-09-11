import Foundation

struct AltitudeModeOffer: Identifiable, Equatable {
    let raw: Int
    let title: String
    let help: String
    let current: Bool
    let enabled: Bool
    let reason: String

    var id: Int { raw }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let raw = (json["raw"] as? NSNumber)?.intValue else { return nil }
        self.raw = raw
        title = (json["title"] as? String) ?? ""
        help = (json["help"] as? String) ?? ""
        current = (json["current"] as? NSNumber)?.boolValue ?? false
        enabled = (json["enabled"] as? NSNumber)?.boolValue ?? false
        reason = (json["reason"] as? String) ?? ""
    }
}

enum AltitudeMode {
    static let mixedRaw = 0
    static let relativeRaw = 1
    static let absoluteRaw = 2
    static let calcAboveTerrainRaw = 3
    static let terrainFrameRaw = 4

    // QGroundControlQmlGlobal::AltMode 5, which QGC names AltitudeModeNone and describes as a
    // "distance value unrelated to ground"; its own menus use it for nothing-selected. It is not
    // a mode an operator picks, and QGC draws it as an empty string.
    static let unrelatedRaw = 5

    // Distinct from the above on purpose: -1 says this head has not been given a value at all,
    // and cannot be confused with a number the enum defines.
    static let none = -1

    static let raws = [mixedRaw, relativeRaw, absoluteRaw, calcAboveTerrainRaw, terrainFrameRaw]

    static let missionContext = "mission"
    static let itemContext = "item"
    static let contexts = [missionContext, itemContext]

    static func read(_ json: Any?) -> Int {
        (json as? NSNumber)?.intValue ?? none
    }

    static func offers(_ json: Any?) -> [AltitudeModeOffer] {
        (((json as? [String: Any])?["modes"] as? [Any]) ?? []).compactMap(AltitudeModeOffer.init)
    }

    static func choosable(_ offers: [AltitudeModeOffer]) -> [AltitudeModeOffer] {
        offers.filter(\.enabled)
    }

    static func refusal(_ offers: [AltitudeModeOffer], raw: Int) -> String? {
        guard let offer = offers.first(where: { $0.raw == raw }) else {
            return "This vehicle does not offer that altitude mode."
        }
        return offer.enabled ? nil : offer.reason
    }

    static func offersPicker(mode raw: Int, in offers: [AltitudeModeOffer]) -> Bool {
        raw != none && !offers.isEmpty
    }

    static func title(for raw: Int, in offers: [AltitudeModeOffer]) -> String {
        guard raw != none, raw != unrelatedRaw else { return "" }
        return offers.first { $0.raw == raw }?.title ?? "Mode \(raw)"
    }
}
