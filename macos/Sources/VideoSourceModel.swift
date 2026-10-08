import Foundation

struct CameraKind: Identifiable, Equatable {
    let raw: String
    let label: String
    let needsUrl: Bool
    let hint: String

    var id: String { raw }

    static func label(for raw: String) -> String {
        raw.replacingOccurrences(of: " Video Stream", with: "")
    }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], let raw = json["raw"] as? String else { return nil }
        self.raw = raw
        label = CameraKind.label(for: (json["label"] as? String) ?? raw)
        needsUrl = (json["needsUrl"] as? NSNumber)?.boolValue ?? false
        hint = (json["hint"] as? String) ?? ""
    }
}

enum CameraSignal: String {
    case live, connecting, noSignal, idle

    var words: String {
        switch self {
        case .live: return "Live"
        case .connecting: return "Connecting"
        case .noSignal: return "No signal"
        case .idle: return "Not playing"
        }
    }
}

struct VideoSource: Identifiable, Equatable {
    let slot: Int
    let stored: Int?
    let title: String
    let short: String
    let name: String
    let source: String
    let url: String
    let summary: String
    let problem: String?
    let fromDrone: Bool
    let active: Bool
    let status: CameraSignal

    var id: Int { slot }

    var caption: String { status == .live ? short : "\(short) \u{00B7} \(status.words)" }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let slot = (json["slot"] as? NSNumber)?.intValue else { return nil }
        self.slot = slot
        stored = (json["stored"] as? NSNumber)?.intValue
        title = (json["title"] as? String) ?? ""
        short = (json["short"] as? String) ?? ""
        name = (json["name"] as? String) ?? ""
        source = (json["source"] as? String) ?? ""
        url = (json["url"] as? String) ?? ""
        summary = (json["summary"] as? String) ?? ""
        problem = (json["problem"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        fromDrone = (json["fromDrone"] as? NSNumber)?.boolValue ?? false
        active = (json["active"] as? NSNumber)?.boolValue ?? false
        status = (json["status"] as? String).flatMap(CameraSignal.init(rawValue:)) ?? .idle
    }
}

struct CameraList: Equatable {
    let readable: Bool
    let reason: String
    let pipEnabled: Bool
    let pipSlot: Int?
    let entries: [VideoSource]
    let kinds: [CameraKind]

    static let empty = CameraList([:])
    static let unanswered = "The camera list change was not answered."

    static func refusal(_ answer: [String: Any]) -> String? {
        guard (answer["ok"] as? NSNumber)?.boolValue != true else { return nil }
        return (answer["reason"] as? String).flatMap { $0.isEmpty ? nil : $0 } ?? unanswered
    }

    init(_ view: [String: Any]) {
        let pip = (view["pip"] as? [String: Any]) ?? [:]
        readable = (view["readable"] as? NSNumber)?.boolValue ?? true
        reason = (view["reason"] as? String) ?? ""
        pipEnabled = (pip["enabled"] as? NSNumber)?.boolValue ?? false
        pipSlot = (pip["slot"] as? NSNumber)?.intValue
        entries = ((view["cameras"] as? [Any]) ?? []).compactMap(VideoSource.init)
        kinds = ((view["kinds"] as? [Any]) ?? []).compactMap(CameraKind.init)
    }

    var offersPip: Bool { pipSlot != nil }

    var pipCamera: VideoSource? { entries.first { pipEnabled && $0.slot == pipSlot } }

    func needsUrl(_ camera: VideoSource) -> Bool {
        kinds.first { $0.raw == camera.source }?.needsUrl ?? true
    }

    func label(_ camera: VideoSource) -> String {
        kinds.first { $0.raw == camera.source }?.label ?? CameraKind.label(for: camera.source)
    }
}

struct CameraGuess: Equatable {
    let address: String
    let kind: String?
    let choices: [String]
    let ambiguous: Bool
    let problem: String?

    static let none = CameraGuess([:])

    init(_ answer: [String: Any]) {
        address = (answer["address"] as? String) ?? ""
        kind = (answer["kind"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        choices = (answer["choices"] as? [String]) ?? []
        ambiguous = (answer["ambiguous"] as? NSNumber)?.boolValue ?? false
        problem = (answer["problem"] as? String).flatMap { $0.isEmpty ? nil : $0 }
    }

    static let whichKind = "Which kind of stream is it?"

    var sentence: String {
        problem ?? (ambiguous ? CameraGuess.whichKind : kind.map(CameraKind.label(for:)) ?? "")
    }

    func choice(keeping picked: String) -> String {
        choices.contains(picked) ? picked : kind ?? ""
    }
}
