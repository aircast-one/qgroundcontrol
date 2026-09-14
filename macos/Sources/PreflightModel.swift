import Foundation

struct PreflightCheck: Identifiable, Equatable {
    // QGC's PreFlightCheckButton has three states, and the core's four verdicts are how it
    // reaches them: a check with no manual text is already passed and is never put to the
    // operator, a telemetry failure that allows an override is pending rather than failed,
    // and one that does not is failed outright.
    enum Verdict: String {
        case manual
        case passing
        case failing
        case overridable
    }

    let name: String
    let prompt: String
    let verdict: Verdict
    let reason: String
    let blocked: Bool

    var id: String { name }

    // A verdict this head does not know must not read as one the core has already cleared,
    // so it falls back to the answer that still asks the operator to look.
    static let unknownVerdict = Verdict.manual

    init?(_ json: Any?) {
        guard let json = json as? [String: Any],
              let name = json["name"] as? String, !name.isEmpty,
              let verdict = json["verdict"] as? String else { return nil }
        self.name = name
        self.verdict = Verdict(rawValue: verdict) ?? PreflightCheck.unknownVerdict
        prompt = (json["prompt"] as? String) ?? ""
        reason = (json["reason"] as? String) ?? ""
        blocked = (json["blocked"] as? NSNumber)?.boolValue ?? false
    }

    var alreadyMet: Bool { verdict == .passing }

    var blocks: Bool { verdict == .failing }

    var warns: Bool { verdict == .overridable }

    var tickable: Bool { !alreadyMet && !blocks }

    // A tick survives the check it was put against: one placed while GPS was merely soft would
    // otherwise still be in the set once the lock dropped and the check began blocking, and the
    // list would call itself ready over a check that stops the flight.
    func met(ticked: Set<String>) -> Bool { !blocks && (alreadyMet || ticked.contains(name)) }

    var symbol: String {
        if blocks { return "exclamationmark.octagon.fill" }
        if warns { return "exclamationmark.triangle.fill" }
        return "circle"
    }

    func symbol(ticked: Set<String>) -> String {
        met(ticked: ticked) ? "checkmark.circle.fill" : symbol
    }

    // The blocking branch used to discard reason and print the generic sentence alone, which is
    // the one case where an operator most needs to know WHAT is wrong: with nothing connected the
    // core says "No vehicle is reporting a battery." and this said "Fix this before it can be
    // checked off", an instruction nobody can act on against an aircraft that is not there. The
    // warning branch was already using reason. Keeping the generic line as the fallback, because
    // a reader must not fail closed when the field it prefers is empty.
    var hint: String {
        if blocks { return reason.isEmpty ? PreflightCheck.blockedWithoutReason : reason }
        if warns { return "\(reason) Check it off to fly anyway." }
        return prompt
    }

    static let blockedWithoutReason = "Fix this before it can be checked off"
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

    // A tick survives contact being LOST, deliberately, and only the vehicle going away clears it.
    // The manual checks ask about the physical aircraft -- props secured, hatch closed, flight area
    // clear -- and none of those answers stops being true because the radio went quiet. Clearing
    // them would make an operator re-answer eleven questions about a machine that has not moved.
    // Readiness while contact is out is a separate question and not this one: the core blocks it.
    static func ticksSurvive(_ previous: FlyState, _ latest: FlyState) -> Bool {
        !(previous.connected && !latest.connected)
    }

    static func total(_ groups: [PreflightGroup]) -> Int {
        groups.reduce(0) { $0 + $1.checks.count }
    }

    static func progress(_ groups: [PreflightGroup], ticked: Set<String>) -> String {
        let settled = groups.flatMap(\.checks).filter { $0.met(ticked: ticked) }
        return "\(settled.count) of \(total(groups)) checked"
    }

    static func ready(_ groups: [PreflightGroup], ticked: Set<String>) -> Bool {
        groups.allSatisfy { group in
            group.checks.allSatisfy { $0.met(ticked: ticked) }
        }
    }
}
