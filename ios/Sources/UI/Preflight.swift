import Foundation

let PREFLIGHT = "view.preflight"

let CHECKLIST_POPUP_DELAY_MS = 1000

struct PreflightCheck: Equatable {
    var name: String
    var prompt: String
    var verdict: String
    var reason: String
    var blocked: Bool
}

struct PreflightGroup: Equatable {
    var name: String
    var checks: [PreflightCheck]
}

struct Preflight: Equatable {
    var airframe: String
    var total: Int
    var blocked: [String]
    var groups: [PreflightGroup]
}

func preflight(_ view: JSON?) -> Preflight? {
    guard let view, let groups = view["groups"].arrayOrNil else { return nil }
    return Preflight(
        airframe: view["airframe"].string,
        total: view["total"].int(0),
        blocked: view["blocked"].strings,
        groups: groups.filter { $0.object != nil }.map { group in
            PreflightGroup(
                name: group["name"].string,
                checks: group["checks"].objects.map {
                    PreflightCheck(
                        name: $0["name"].string,
                        prompt: $0["prompt"].string,
                        verdict: $0["verdict"].string,
                        reason: $0["reason"].string,
                        blocked: $0["blocked"].bool
                    )
                }
            )
        }
    )
}

func checklistOffered(_ armed: Bool) -> String? { armed ? "The checks are for before the flight" : nil }

func preflightOffered(_ view: JSON?) -> Bool { view?["offered"].bool(true) ?? true }

func preflightSummary(_ preflight: Preflight?, _ ticked: Set<String>) -> String {
    guard let preflight else { return "Connect a vehicle to run its preflight checks." }
    let checks = preflight.groups.flatMap(\.checks)
    let manual = Set(checks.filter(checkNeedsTicking).map(\.name))
    let outstanding = manual.subtracting(ticked).count
    let warnings = checks.filter { $0.verdict == "overridable" }.count
    if !preflight.blocked.isEmpty {
        let left = outstanding > 0 ? " \u{00B7} \(outstanding) left to check" : ""
        return "\(preflight.blocked.count) of \(preflight.total) will stop the flight\(left)."
    }
    if outstanding > 0 { return "\(outstanding) of \(manual.count) left to check." }
    if warnings > 0 { return "All \(preflight.total) checks done \u{00B7} \(warnings) warning\(warnings == 1 ? "" : "s")." }
    return "All \(preflight.total) checks done."
}

func groupPassed(_ group: PreflightGroup, _ ticked: Set<String>) -> Bool {
    group.checks.allSatisfy { !$0.blocked && (!checkNeedsTicking($0) || ticked.contains($0.name)) }
}

func groupEnabled(_ groups: [PreflightGroup], _ index: Int, _ ticked: Set<String>) -> Bool {
    groups.prefix(index).allSatisfy { groupPassed($0, ticked) }
}

func groupHeading(_ group: PreflightGroup, _ ticked: Set<String>) -> String {
    groupPassed(group, ticked) ? "\(group.name) (passed)" : group.name
}

let GROUP_COLLAPSE_DELAY_MS = 750

func collapsedAfterPass(_ collapsed: Set<String>, _ passedBefore: Set<String>, _ passedNow: Set<String>) -> Set<String> {
    collapsed.union(passedNow.subtracting(passedBefore))
}

func checklistHeading(_ passed: Bool) -> String { "Pre-flight checklist \(passed ? "(passed)" : "in progress")" }

func checklistIsComplete(_ preflight: Preflight?, _ ticked: Set<String>) -> Bool {
    guard let preflight, preflight.blocked.isEmpty else { return false }
    return preflight.groups.flatMap(\.checks).filter(checkNeedsTicking).allSatisfy { ticked.contains($0.name) }
}

func checklistPopupIsDue(
    _ hasVehicle: Bool,
    _ useChecklist: Bool,
    _ enforceChecklist: Bool,
    _ complete: Bool,
    deciding: Bool = false
) -> Bool {
    hasVehicle && useChecklist && enforceChecklist && !complete && !deciding
}

func checkNeedsTicking(_ check: PreflightCheck) -> Bool { check.verdict == "manual" || check.verdict == "overridable" }

enum CheckMark { case TICKABLE, PASSED, ATTENTION }

func checkMark(_ check: PreflightCheck) -> CheckMark {
    checkNeedsTicking(check) ? .TICKABLE : check.verdict == "passing" ? .PASSED : .ATTENTION
}

func checkStatusText(_ check: PreflightCheck, _ ticked: Bool) -> String {
    switch check.verdict {
    case "passing": "Passing"
    case "failing": check.reason.ifBlank("Failing")
    case "overridable": ticked ? "Checked" : check.reason.ifBlank("Needs attention")
    default: ticked ? "Checked" : "Check it"
    }
}
