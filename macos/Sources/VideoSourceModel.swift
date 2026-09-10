import Foundation

struct VideoSource: Identifiable, Equatable {
    let slot: Int
    var name: String
    var source: String
    var url: String
    var enabled: Bool
    var configured: Bool

    var id: Int { slot }

    var misconfigured: Bool { enabled && !configured }

    var summary: String {
        guard enabled else { return "Off" }
        if !configured { return "No address" }
        return url.isEmpty ? source : url
    }

    var title: String { name.isEmpty ? "Camera \(slot + 1)" : name }
}

enum VideoSources {
    // VideoSettings numbers slot 0 as the main videoSource fact and the extra sources from 1, so
    // this list of extras sits one behind view.video.cameras.
    static let firstExtraSlot = 1

    static func decode(_ json: String, cameras: [VideoCamera] = []) -> [VideoSource] {
        guard let data = json.data(using: .utf8),
              let listed = (try? JSONSerialization.jsonObject(with: data)) as? [[String: Any]]
        else { return [] }
        let bySlot = Dictionary(cameras.map { ($0.slot, $0) }, uniquingKeysWith: { first, _ in first })
        return listed.enumerated().map { slot, entry in
            let camera = bySlot[slot + VideoSources.firstExtraSlot]
            return VideoSource(slot: slot,
                               name: (entry["name"] as? String) ?? "",
                               source: (entry["source"] as? String) ?? "",
                               url: (entry["url"] as? String) ?? "",
                               enabled: camera?.enabled ?? true,
                               configured: camera?.configured ?? true)
        }
    }

    static func encode(_ sources: [VideoSource]) -> String {
        let listed = sources.map { ["name": $0.name, "source": $0.source, "url": $0.url] }
        guard let data = try? JSONSerialization.data(withJSONObject: listed),
              let text = String(data: data, encoding: .utf8)
        else { return "[]" }
        return text
    }

    static func looksLikeAddress(_ text: String) -> Bool {
        guard !text.isEmpty, !text.contains(" ") else { return false }
        if text.contains("://") { return true }
        guard let colon = text.lastIndex(of: ":") else { return false }
        let port = text[text.index(after: colon)...]
        return !port.isEmpty && port.allSatisfy(\.isNumber)
    }

    static func repairs(_ source: VideoSource) -> VideoSource? {
        guard source.misconfigured, looksLikeAddress(source.name) else { return nil }
        var repaired = source
        repaired.url = source.name
        repaired.name = ""
        return repaired
    }

    static func replacing(_ sources: [VideoSource], at slot: Int,
                          with replacement: VideoSource) -> [VideoSource] {
        sources.map { $0.slot == slot ? replacement : $0 }
    }
}
