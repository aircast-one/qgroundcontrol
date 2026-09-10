import Foundation

struct VideoSource: Identifiable, Equatable {
    let slot: Int
    var name: String
    var source: String
    var url: String

    var id: Int { slot }

    static let disabled = "Video Stream Disabled"

    // REPORTED, NOT FIXED: VideoSettings declares videoDisabled with QT_TRANSLATE_NOOP, so this
    // compares a stored source against English text. Under another language a disabled source
    // reads as enabled, is then called misconfigured for having no address, and is offered for
    // repair. The stable answer is VideoSettings::sourceConfigured, which the core can reach and
    // this head cannot; asked for it rather than papering over it here.
    var enabled: Bool { !source.isEmpty && source != VideoSource.disabled }

    var misconfigured: Bool { enabled && url.isEmpty }

    var summary: String {
        guard enabled else { return "Off" }
        if url.isEmpty { return "No address" }
        return url
    }

    var title: String { name.isEmpty ? "Camera \(slot + 1)" : name }
}

enum VideoSources {
    static func decode(_ json: String) -> [VideoSource] {
        guard let data = json.data(using: .utf8),
              let listed = (try? JSONSerialization.jsonObject(with: data)) as? [[String: Any]]
        else { return [] }
        return listed.enumerated().map { slot, entry in
            VideoSource(slot: slot,
                        name: (entry["name"] as? String) ?? "",
                        source: (entry["source"] as? String) ?? "",
                        url: (entry["url"] as? String) ?? "")
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
