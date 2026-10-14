import XCTest
@testable import Aircast

final class GuidedActionsTests: XCTestCase {
    private let served = #"""
        {"connected":true,"forwardFlight":false,"missionActive":false,"actions":[
          {"id":"arm","title":"Arm","offer":"blocked","reason":"Vehicle is not ready to arm.",
           "prompt":"Arming spins the propellers.","destructive":true,"carriesValue":false},
          {"id":"takeoff","title":"Takeoff","offer":"ready","reason":"",
           "prompt":"The vehicle will climb.","destructive":false,"carriesValue":true},
          {"id":"land","title":"Land","offer":"hidden","reason":"",
           "prompt":"","destructive":false,"carriesValue":false}]}
        """#

    func testAnOfferCarriesItsStateReasonAndPrompt() {
        let offers = guidedOffers(JSON.parse(served))
        XCTAssertFalse(offers["arm"]!.ready)
        XCTAssertTrue(offers["takeoff"]!.ready)
        XCTAssertFalse(offers["land"]!.shown)
        XCTAssertEqual(offers["arm"]!.reason, "Vehicle is not ready to arm.")
        XCTAssertEqual(offers["takeoff"]!.prompt, "The vehicle will climb.")
        XCTAssertTrue(offers["arm"]!.destructive)
    }

    func testADisarmTheCoreMarksUnconfirmedGoesOutOnTheTapEverythingElseStillAsks() {
        let offers = guidedOffers(JSON.parse(#"""
            {"actions":[
              {"id":"disarm","title":"Disarm","offer":"ready","prompt":"Disarm the vehicle","destructive":false,"confirm":false,"carriesValue":false},
              {"id":"land","title":"Land","offer":"ready","prompt":"Land","destructive":false,"confirm":true,"carriesValue":false},
              {"id":"rtl","title":"Return","offer":"ready","prompt":"Return","destructive":false,"carriesValue":false}]}
            """#))
        let action = { (id: String) in GuidedAction(name: id, confirm: "", destructive: false, offerId: id, run: {}) }
        XCTAssertFalse(needsConfirm(action("disarm"), offers))
        XCTAssertTrue(needsConfirm(action("land"), offers))
        XCTAssertTrue(needsConfirm(action("rtl"), offers), "an offer that does not say asks first")
        XCTAssertTrue(needsConfirm(action("gone"), offers), "an action with no offer behind it asks first")
    }

    func testNoViewIsNoOffersRatherThanACrash() {
        XCTAssertEqual(guidedOffers(nil), [:])
        XCTAssertEqual(guidedOffers(JSON.parse("{}")), [:])
    }
}
