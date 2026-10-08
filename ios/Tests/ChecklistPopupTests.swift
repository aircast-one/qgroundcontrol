import XCTest
@testable import Aircast

final class ChecklistPopupTests: XCTestCase {
    private func served(_ checks: String..., blocked: Int = 0) -> Preflight? {
        let stops = (0..<blocked).map { "\"stop \($0)\"" }.joined(separator: ",")
        return preflight(JSON.parse(
            #"{"kind":"object","class":"Preflight","total":\#(checks.count),"#
                + #""blocked":[\#(stops)],"#
                + #""groups":[{"name":"Before flight","checks":[\#(checks.joined(separator: ","))]}]}"#
        ))
    }

    private func manual(_ name: String) -> String { #"{"name":"\#(name)","verdict":"manual","text":"\#(name)"}"# }
    private func automatic(_ name: String) -> String { #"{"name":"\#(name)","verdict":"pass","text":"\#(name)"}"# }

    func testAnUntickedManualCheckLeavesTheListIncomplete() {
        XCTAssertFalse(checklistIsComplete(served(manual("Props on")), []))
        XCTAssertTrue(checklistIsComplete(served(manual("Props on")), ["Props on"]))
    }

    func testABlockingCheckIsNeverCompleteHoweverMuchIsTicked() {
        XCTAssertFalse(
            checklistIsComplete(served(manual("Props on"), blocked: 1), ["Props on"]),
            "a blocker stops the flight; ticking the manual items around it does not clear it"
        )
    }

    func testAListWithNothingToTickIsAlreadyComplete() {
        XCTAssertTrue(checklistIsComplete(served(automatic("Battery")), []))
    }

    func testNoChecklistAtAllIsNotComplete() {
        XCTAssertFalse(
            checklistIsComplete(nil, []),
            "absent is not passed - with no vehicle there is nothing to have checked"
        )
    }

    func testThePopupNeverOpensOverADecisionInProgress() {
        XCTAssertFalse(
            checklistPopupIsDue(true, true, true, false, deciding: true),
            "a hold-to-confirm or a value panel is the operator mid-decision; a checklist "
                + "landing on top of it takes the screen at the worst moment"
        )
        XCTAssertTrue(
            checklistPopupIsDue(true, true, true, false, deciding: false),
            "and it is owed once they are done, not cancelled - the effect re-keys on the "
                + "flag so closing the decision brings it back"
        )
    }

    func testThePopupIsDueOnlyWhenEveryFlagQtReadsIsSet() {
        XCTAssertTrue(checklistPopupIsDue(true, true, true, false))
        XCTAssertFalse(checklistPopupIsDue(false, true, true, false), "no vehicle, nothing to check")
        XCTAssertFalse(checklistPopupIsDue(true, false, true, false), "the operator turned the checklist off")
        XCTAssertFalse(checklistPopupIsDue(true, true, false, false), "enforcement off means the list is available but not pushed")
        XCTAssertFalse(checklistPopupIsDue(true, true, true, true), "already done, so opening it would interrupt for nothing")
    }

    func testGroupsUnlockInOrderAndSayWhenTheyPassLikePreFlightCheckModelEnforceOrder() {
        let first = PreflightGroup(name: "Initial checks", checks: [PreflightCheck(name: "hardware", prompt: "", verdict: "manual", reason: "", blocked: false)])
        let second = PreflightGroup(name: "Please arm the vehicle here", checks: [PreflightCheck(name: "arm", prompt: "", verdict: "manual", reason: "", blocked: false)])
        let groups = [first, second]
        XCTAssertEqual(groups.indices.map { groupEnabled(groups, $0, []) }, [true, false])
        XCTAssertEqual(groups.indices.map { groupEnabled(groups, $0, ["hardware"]) }, [true, true])
        XCTAssertEqual(groupHeading(first, ["hardware"]), "Initial checks (passed)")
        XCTAssertEqual(groupHeading(second, ["hardware"]), "Please arm the vehicle here")
        XCTAssertEqual(checklistHeading(true), "Pre-flight checklist (passed)")
        XCTAssertEqual(checklistHeading(false), "Pre-flight checklist in progress")
    }

    func testOnlyAGroupThatHasJustPassedCollapsesLikePreFlightCheckListHandleGroupPassedChanged() {
        XCTAssertEqual(collapsedAfterPass([], [], ["Initial checks"]), ["Initial checks"])
        XCTAssertEqual(
            collapsedAfterPass([], ["Initial checks"], ["Initial checks", "Arm"]),
            ["Arm"],
            "a group the operator reopened stays open when another passes"
        )
    }
}
