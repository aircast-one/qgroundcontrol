import Foundation

struct InterceptConfirm: Equatable {
    let title: String
    let text: String
}

struct InterceptAction: Identifiable, Equatable {
    let id: String
    let label: String
    let path: String
    let args: [Any]
    let confirm: InterceptConfirm?

    init?(_ json: Any?) {
        guard let json = json as? [String: Any] else { return nil }
        id = (json["id"] as? String) ?? ""
        label = (json["label"] as? String) ?? ""
        path = (json["path"] as? String) ?? ""
        args = (json["args"] as? [Any]) ?? []
        confirm = (json["confirm"] as? [String: Any]).map {
            InterceptConfirm(title: ($0["title"] as? String) ?? "", text: ($0["text"] as? String) ?? "")
        }
    }

    static func == (lhs: InterceptAction, rhs: InterceptAction) -> Bool {
        lhs.id == rhs.id && lhs.label == rhs.label && lhs.path == rhs.path
            && lhs.confirm == rhs.confirm && (lhs.args as NSArray).isEqual(to: rhs.args)
    }
}

struct AutoIntercept: Equatable {
    let visible: Bool
    let problem: Bool
    let headline: String
    let actions: [InterceptAction]
    let entry: InterceptAction?
    let hint: String?
    let picking: Bool
    let pickPath: String?

    init?(_ json: [String: Any]) {
        guard (json["class"] as? String) == "AutoIntercept" else { return nil }
        visible = (json["visible"] as? NSNumber)?.boolValue ?? false
        problem = (json["tone"] as? String) == "problem"
        headline = (json["headline"] as? String) ?? ""
        actions = ((json["actions"] as? [Any]) ?? []).compactMap(InterceptAction.init)
        entry = InterceptAction(json["entry"])
        hint = json["hint"] as? String
        picking = (json["picking"] as? NSNumber)?.boolValue ?? false
        pickPath = json["pickPath"] as? String
    }

    var offered: [InterceptAction] { actions + [entry].compactMap { $0 } }
}
