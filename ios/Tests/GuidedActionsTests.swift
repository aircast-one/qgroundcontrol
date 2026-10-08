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

    func testNoViewIsNoOffersRatherThanACrash() {
        XCTAssertEqual(guidedOffers(nil), [:])
        XCTAssertEqual(guidedOffers(JSON.parse("{}")), [:])
    }
}
