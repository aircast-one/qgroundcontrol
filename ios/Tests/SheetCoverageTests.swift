import XCTest
@testable import Aircast

final class SheetCoverageTests: XCTestCase {
    private let everyServedAction = [
        "arm", "takeoff", "startMission", "continueMission", "resumeMission",
        "cancelRoi", "pause", "changeAltitude", "changeSpeed", "landAbort",
        "land", "rtl", "disarm", "grab", "release", "emergencyStop",
        "vtolTransitionToFixedWing", "vtolTransitionToMultiRotor", "forceArm",
    ]

    private func allReady() -> [String: GuidedOffer] {
        let actions = everyServedAction.map { id in
            #"{"id":"\#(id)","title":"\#(id)","offer":"ready","reason":"","prompt":"","destructive":false,"carriesValue":false}"#
        }.joined(separator: ",")
        return guidedOffers(JSON.parse(#"{"actions":[\#(actions)]}"#))
    }

    func testTheBarAndTheSheetNeverOfferTheSameAction() {
        XCTAssertEqual(
            BAR_ACTIONS.intersection(SHEET_ACTIONS),
            [],
            "an action in both places is two buttons for one command, and the operator cannot tell which one the vehicle heard"
        )
    }

    func testEveryActionTheCoreCanServeIsReachableSomewhere() {
        let unreachable = everyServedAction.filter { !BAR_ACTIONS.contains($0) && !SHEET_ACTIONS.contains($0) && $0 != EMERGENCY_STOP }
        XCTAssertEqual(
            unreachable,
            [],
            "a served action drawn in neither place is one the operator can never send; the VTOL transitions were exactly that until this commit"
        )
    }

    func testTheSheetHoldsEveryReadyActionThatIsNotOnTheBar() {
        let shown = Set(moreActions(allReady()).map(\.id))
        XCTAssertTrue(
            shown.contains("vtolTransitionToFixedWing") && shown.contains("vtolTransitionToMultiRotor"),
            "the transitions are how a VTOL changes between hover and forward flight, and this head drew neither"
        )
        XCTAssertTrue(shown.contains("forceArm"))
        XCTAssertTrue(shown.allSatisfy { !BAR_ACTIONS.contains($0) }, "nothing on the bar may also be in the sheet")
        XCTAssertFalse(shown.contains(EMERGENCY_STOP), "the pinned stop is never in the sheet")
    }

    func testEveryActionTheSheetOffersCanActuallyBeSent() {
        moreActions(allReady()).filter { $0.id != PAUSE }.forEach { offer in
            XCTAssertNotNil(guidedCommand(offer.id, 3), "\(offer.id) is drawn in the sheet but this head has no way to send it")
        }
    }
}
