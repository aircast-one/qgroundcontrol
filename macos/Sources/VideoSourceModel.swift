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

    // decode returns [] for a corrupt setting AND for no extra sources, and those are not the same
    // fact. Told apart nowhere, an unparseable string presents as a fresh install -- and because
    // write() encodes whatever list it is holding, the very next edit persists [] OVER the original
    // text. A display bug becomes a data-loss bug at that line, so the store must know which case
    // it is in before it is allowed to write.
    //
    // Valid JSON that is not an array is unreadable too: {"name":"Nose"} is not a list of sources.
    // The core's answer is preferred and the head's parse is the fallback, in that order. One
    // parser deciding for both heads is the point of serving it; a head that cannot ask an older
    // core still has to know, because the write gate depends on the answer and a gate that fails
    // open on a missing field is not a gate.
    static func readability(_ extra: Any?, stored fallback: String) -> (readable: Bool, stored: String) {
        guard let extra = extra as? [String: Any],
              let served = (extra["readable"] as? NSNumber)?.boolValue else {
            return (readable(fallback), fallback)
        }
        return (served, (extra["stored"] as? String) ?? fallback)
    }

    static let unreadable = "The saved video sources could not be read, so they have not been "
        + "changed. Editing them now would replace what is stored."

    static func readable(_ json: String) -> Bool {
        guard !json.isEmpty else { return true }
        guard let data = json.data(using: .utf8),
              let parsed = try? JSONSerialization.jsonObject(with: data) else { return false }
        return parsed is [Any]
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
