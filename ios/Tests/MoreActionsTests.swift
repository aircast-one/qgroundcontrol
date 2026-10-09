import XCTest
@testable import Aircast

final class MoreActionsTests: XCTestCase {
    private func offers(_ entries: (String, String)...) -> [String: GuidedOffer] {
        let actions = entries.map { id, offer in
            #"{"id":"\#(id)","title":"\#(id)","offer":"\#(offer)","reason":"","prompt":"","destructive":false,"carriesValue":false}"#
        }.joined(separator: ",")
        return guidedOffers(JSON.parse(#"{"actions":[\#(actions)]}"#))
    }

    private func offers(_ entries: [(String, String)]) -> [String: GuidedOffer] {
        let actions = entries.map { id, offer in
            #"{"id":"\#(id)","title":"\#(id)","offer":"\#(offer)","reason":"","prompt":"","destructive":false,"carriesValue":false}"#
        }.joined(separator: ",")
        return guidedOffers(JSON.parse(#"{"actions":[\#(actions)]}"#))
    }

    func testAnRoiLockTheVehicleIsHoldingCanBeReleasedFromThisHead() {
        XCTAssertEqual(
            moreActions(offers(("cancelRoi", "ready"))).map(\.id),
            ["cancelRoi"],
            "the core offers CancelRoi only on roi_supported && roi_active && flying, so the sheet showing it means a lock IS in force"
        )
        XCTAssertNotNil(guidedCommand("cancelRoi", nil))
    }

    func testResumingAMissionCarriesTheWaypointTheCoreHoldsNeverOneTheHeadCounted() {
        XCTAssertNotNil(guidedCommand("resumeMission", 5))
        XCTAssertEqual(moreActions(offers(("resumeMission", "ready"))).map(\.id), ["resumeMission"])
    }

    func testWithoutTheWaypointThereIsNoResumeCommandToRun() {
        XCTAssertNil(guidedCommand("resumeMission", nil), "refusing to send rather than sending a resume to waypoint zero")
    }

    func testTheWaypointIsReadFromTheCoreAndAWithheldOneIsNotAZero() {
        XCTAssertEqual(resumeFromSequence(JSON.parse(#"{"resumeFromSequence":5}"#)), 5)
        XCTAssertNil(resumeFromSequence(JSON.parse(#"{"resumeFromSequence":null}"#)))
        XCTAssertNil(resumeFromSequence(JSON.parse(#"{}"#)))
        XCTAssertNil(resumeFromSequence(JSON.parse(#"{"resumeFromSequence":0}"#)), "a resume sent to waypoint zero would fly the plan again from the start")
        XCTAssertNil(resumeFromSequence(nil))
    }

    func testTheSheetCarriesWhatTheRowDoesNot() {
        let extra = moreActions(offers(("arm", "ready"), ("takeoff", "ready"), ("startMission", "ready"), ("pause", "blocked"), ("landAbort", "ready")))
        XCTAssertEqual(extra.map(\.id), ["startMission", "pause", "landAbort"])
    }

    func testTheSheetKeepsTheOrderTheCoreSendsTheOffersIn() {
        let extra = moreActions(offers(("forceArm", "ready"), ("landAbort", "ready"), ("startMission", "ready")))
        XCTAssertEqual(extra.map(\.id), ["forceArm", "landAbort", "startMission"])
    }

    func testTheGripperPanelListsReleaseGrabHoldInQgcsOrder() {
        let extra = moreActions(offers(("hold", "ready"), ("grab", "ready"), ("release", "disabled"), ("startMission", "ready")))
        XCTAssertEqual(gripperOffers(extra).map(\.id), ["release", "grab", "hold"])
        XCTAssertEqual(gripperOffers(moreActions(offers(("grab", "hidden")))).map(\.id), [])
    }

    func testAnActionTheCoreHidesIsNotOffered() {
        XCTAssertEqual(moreActions(offers(("grab", "hidden"), ("release", "ready"))).map(\.id), ["release"])
    }

    func testAnActionThisHeadCannotSendIsNotOffered() {
        XCTAssertEqual(moreActions(offers(("orbit", "ready"))).map(\.id), [])
    }

    func testEveryActionTheSheetShowsCanBeSentExceptPauseWhichAsksForAHeight() {
        let ids = moreActions(offers((SHEET_ACTIONS + [EMERGENCY_STOP]).map { ($0, "ready") })).map(\.id)
        XCTAssertEqual(ids.count, SHEET_ACTIONS.count, "every id in SHEET_ACTIONS, so adding one to the set without a command here is caught")
        ids.filter { $0 != PAUSE }.forEach { XCTAssertNotNil(guidedCommand($0, 5), $0) }
        XCTAssertNil(guidedCommand(PAUSE, 5))
    }

    private func offer(_ id: String, _ offer: String) -> GuidedOffer {
        GuidedOffer(id: id, title: id, offer: offer, reason: "", prompt: "", destructive: false, carriesValue: false)
    }

    func testStartMissionPopsUpTheMomentItBecomesAvailableAndNotAgain() {
        let ready = ["startMission": offer("startMission", "ready")]
        XCTAssertEqual(autoMissionPopup([], ready, true)?.id, "startMission")
        XCTAssertNil(autoMissionPopup(["startMission"], ready, true))
        XCTAssertNil(autoMissionPopup([], ready, false))
        XCTAssertNil(autoMissionPopup([], ["startMission": offer("startMission", "blocked")], true))
    }

    func testLandAbortPopsUpOnAFixedWingApproachEvenWithMissionPopupsOff() {
        let approach = ["landAbort": offer("landAbort", "ready")]
        XCTAssertEqual(autoMissionPopup([], approach, false)?.id, "landAbort", "GuidedActionsController's onShowLandAbortChanged confirms it whatever enableAutomaticMissionPopups says")
        XCTAssertNil(autoMissionPopup(["landAbort"], approach, false))
        XCTAssertTrue(popupReplacesOpenConfirm("landAbort"), "confirmAction(actionLandAbort) replaces whatever confirm is open")
        XCTAssertFalse(popupReplacesOpenConfirm("startMission"))
    }

    func testContinueMissionPopsUpToo() {
        XCTAssertEqual(autoMissionPopup([], ["continueMission": offer("continueMission", "ready")], true)?.id, "continueMission")
    }
}
