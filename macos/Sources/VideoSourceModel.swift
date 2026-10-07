import Foundation

struct CameraKind: Identifiable, Equatable {
    let raw: String
    let label: String
    let needsUrl: Bool
    let hint: String

    var id: String { raw }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any], let raw = json["raw"] as? String else { return nil }
        self.raw = raw
        label = ((json["label"] as? String) ?? raw).replacingOccurrences(of: " Video Stream", with: "")
        needsUrl = (json["needsUrl"] as? NSNumber)?.boolValue ?? false
        hint = (json["hint"] as? String) ?? ""
    }
}

struct VideoSource: Identifiable, Equatable {
    let slot: Int
    let stored: Int?
    let title: String
    let name: String
    let source: String
    let url: String
    let summary: String
    let problem: String?
    let fromDrone: Bool
    let active: Bool

    var id: Int { slot }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let slot = (json["slot"] as? NSNumber)?.intValue else { return nil }
        self.slot = slot
        stored = (json["stored"] as? NSNumber)?.intValue
        title = (json["title"] as? String) ?? ""
        name = (json["name"] as? String) ?? ""
        source = (json["source"] as? String) ?? ""
        url = (json["url"] as? String) ?? ""
        summary = (json["summary"] as? String) ?? ""
        problem = (json["problem"] as? String).flatMap { $0.isEmpty ? nil : $0 }
        fromDrone = (json["fromDrone"] as? NSNumber)?.boolValue ?? false
        active = (json["active"] as? NSNumber)?.boolValue ?? false
    }
}

struct CameraList: Equatable {
    let readable: Bool
    let reason: String
    let entries: [VideoSource]
    let kinds: [CameraKind]

    static let empty = CameraList([:])

    init(_ view: [String: Any]) {
        readable = (view["readable"] as? NSNumber)?.boolValue ?? true
        reason = (view["reason"] as? String) ?? ""
        entries = ((view["cameras"] as? [Any]) ?? []).compactMap(VideoSource.init)
        kinds = ((view["kinds"] as? [Any]) ?? []).compactMap(CameraKind.init)
    }

    func needsUrl(_ camera: VideoSource) -> Bool {
        kinds.first { $0.raw == camera.source }?.needsUrl ?? true
    }
}
