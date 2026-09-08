import Foundation

struct PreflightCheck: Identifiable, Equatable {
    let name: String
    let prompt: String
    let verdict: String
    let reason: String
    let blocked: Bool

    var id: String { name }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty,
              let verdict = json["verdict"] as? String else { return nil }
        self.name = name
        self.verdict = verdict
        prompt = (json["prompt"] as? String) ?? ""
        reason = (json["reason"] as? String) ?? ""
        blocked = (json["blocked"] as? NSNumber)?.boolValue ?? false
    }
}

struct PreflightGroup: Identifiable, Equatable {
    let name: String
    let checks: [PreflightCheck]

    var id: String { name }

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty else { return nil }
        self.name = name
        checks = ((json["checks"] as? [Any]) ?? []).compactMap(PreflightCheck.init)
    }
}

enum Preflight {
    static func groups(_ json: Any?) -> [PreflightGroup] {
        ((json as? [Any]) ?? []).compactMap(PreflightGroup.init)
    }

    static func total(_ groups: [PreflightGroup]) -> Int {
        groups.reduce(0) { $0 + $1.checks.count }
    }

    static func progress(_ groups: [PreflightGroup], ticked: Set<String>) -> String {
        let present = Set(groups.flatMap(\.checks).map(\.name))
        return "\(ticked.intersection(present).count) of \(total(groups)) checked"
    }

    static func ready(_ groups: [PreflightGroup], ticked: Set<String>) -> Bool {
        groups.allSatisfy { group in
            group.checks.allSatisfy { ticked.contains($0.name) }
        }
    }
}
