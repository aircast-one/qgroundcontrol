import XCTest
@testable import Aircast

final class GuidedTakeoffTests: XCTestCase {
    func testTheServedTakeoffCarriesItsRangeItsSentenceAndTheMetricTarget() {
        let takeoff = guidedTakeoff(JSON.parse(#"""
            {"available":true,"label":"Takeoff altitude","unit":"m",
            "initial":10.0,"minimum":3.0,"maximum":121.0,"minimumMeters":3.0,
            "target":10.0,"targetMeters":10.0,
            "sentence":"The aircraft will climb to 10.0 m and hold."}
            """#))!
        XCTAssertEqual(takeoff.initial!, 10.0, accuracy: 1e-6)
        XCTAssertEqual(takeoff.targetMeters, 10.0, accuracy: 1e-6)
        XCTAssertEqual(takeoff.sentence, "The aircraft will climb to 10.0 m and hold.")
        XCTAssertTrue(takeoffRangeUsable(takeoff))
    }

    func testAVehicleWithNoRangeYetOffersNoSlider() {
        let takeoff = guidedTakeoff(JSON.parse(#"""
            {"available":true,"label":"Takeoff altitude","unit":"m",
            "initial":null,"minimum":null,"maximum":null,"minimumMeters":null}
            """#))!
        XCTAssertNil(takeoff.initial)
        XCTAssertFalse(takeoffRangeUsable(takeoff))
    }

    func testAnUnavailableTakeoffIsNoDialog() {
        XCTAssertNil(guidedTakeoff(nil))
        XCTAssertNil(guidedTakeoff(JSON.parse(#"{"available":false}"#)))
        XCTAssertFalse(takeoffRangeUsable(nil))
    }

    func testTheArgumentPathCarriesTheTargetAndNoSeparators() {
        XCTAssertEqual(guidedTakeoffPath(10.0), "view.guidedTakeoff(10.00)")
        XCTAssertFalse(guidedTakeoffPath(10.0).contains(","))
    }

    func testHoldingTakeoffClimbsToTheDefaultHeightAndRefusesAVehicleWithNoRange() {
        func takeoff(_ initial: Double?, _ minimum: Double?, _ maximum: Double?) -> GuidedTakeoff {
            GuidedTakeoff(label: "Takeoff altitude", unit: "m", initial: initial, minimum: minimum, maximum: maximum, sentence: "", targetMeters: 10.0)
        }
        XCTAssertEqual(holdTakeoffHeight(takeoff(10.0, 3.0, 121.0))!, 10.0, accuracy: 0)
        XCTAssertNil(holdTakeoffHeight(takeoff(10.0, nil, 121.0)))
        XCTAssertNil(holdTakeoffHeight(takeoff(10.0, 5.0, 5.0)))
        XCTAssertNil(holdTakeoffHeight(nil))
    }
}
