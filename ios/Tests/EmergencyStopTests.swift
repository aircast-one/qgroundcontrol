import XCTest
@testable import Aircast

final class EmergencyStopTests: XCTestCase {
    private func offers(_ entries: (String, String)...) -> [String: GuidedOffer] {
        let actions = entries.map { id, offer in
            #"{"id":"\#(id)","title":"\#(id)","offer":"\#(offer)","reason":"","prompt":"","destructive":false,"carriesValue":false}"#
        }.joined(separator: ",")
        return guidedOffers(JSON.parse(#"{"actions":[\#(actions)]}"#))
    }

    func testHoldToDisarmWhileFlyingBecomesTheEmergencyStop() {
        let flying = offers((EMERGENCY_STOP, "ready"), ("disarm", "hidden"))
        XCTAssertNotNil(armedStopOffer(flying, true))
        XCTAssertNil(armedStopOffer(flying, false))
        XCTAssertNil(armedStopOffer(offers((EMERGENCY_STOP, "hidden")), true))
    }

    func testTheEmergencyStopIsNeverInTheOverflowSheet() {
        XCTAssertFalse(SHEET_ACTIONS.contains(EMERGENCY_STOP))
    }

    func testAReadyEmergencyStopAppearsInNoSheetListing() {
        let extras = moreActions(offers((EMERGENCY_STOP, "ready"), ("grab", "ready")))
        XCTAssertTrue(extras.allSatisfy { $0.id != EMERGENCY_STOP })
        XCTAssertEqual(extras.map(\.id), ["grab"])
    }

    func testThePinnedButtonTakesTheOfferTheCoreServes() {
        XCTAssertNotNil(emergencyStopOffer(offers((EMERGENCY_STOP, "ready"))))
        XCTAssertNotNil(emergencyStopOffer(offers((EMERGENCY_STOP, "blocked"))))
    }

    func testAHiddenEmergencyStopDrawsNothing() {
        XCTAssertNil(emergencyStopOffer(offers((EMERGENCY_STOP, "hidden"))))
        XCTAssertNil(emergencyStopOffer(offers(("grab", "ready"))))
    }

    func testTheConfirmationIsAlwaysDestructiveWhateverTheCoreSays() {
        let offer = offers((EMERGENCY_STOP, "ready"))[EMERGENCY_STOP]!
        XCTAssertFalse(offer.destructive)
        XCTAssertTrue(emergencyStopAction(offer).destructive)
    }
}
