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

    static let unrelatedRaw = 5

    static let none = -1

    static let raws = [mixedRaw, relativeRaw, absoluteRaw, calcAboveTerrainRaw, terrainFrameRaw]

    static let missionContext = "mission"
    static let itemContext = "item"
    static let contexts = [missionContext, itemContext]

    static func read(_ json: Any?) -> Int {
        (json as? NSNumber)?.intValue ?? none
    }

    static func offers(_ json: Any?) -> [AltitudeModeOffer] {
        let read = json as? [String: Any]
        let served = ((read?["modes"] as? [Any]) ?? []).compactMap(AltitudeModeOffer.init)
        let removed = ((read?["omitted"] as? [Any]) ?? []).compactMap(AltitudeModeOffer.init)
        return served + removed
    }

    static func choosable(_ offers: [AltitudeModeOffer]) -> [AltitudeModeOffer] {
        offers.filter(\.enabled)
    }

    static func refusalNote(_ offers: [AltitudeModeOffer]) -> String? {
        let reasons = offers.filter { !$0.enabled && !$0.reason.isEmpty }.map(\.reason)
        let distinct = reasons.reduce(into: [String]()) { kept, reason in
            guard !kept.contains(reason) else { return }
            kept.append(reason)
        }
        return distinct.isEmpty ? nil : distinct.joined(separator: " ")
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
