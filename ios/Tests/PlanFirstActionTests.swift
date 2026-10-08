import XCTest
@testable import Aircast

final class PlanFirstActionTests: XCTestCase {
    private func emptyPlanText(_ canAddByHand: Bool, _ canPlaceByButton: Bool) -> String {
        planSummary(0, [], [], [], [], [], [], "", nil, offline: true, canAddByHand: canAddByHand, canPlaceByButton: canPlaceByButton)
    }

    func testWithATakeoffRequiredAndNowhereToPutItNeitherAffordanceWorks() {
        XCTAssertEqual("Empty plan · move the map to where you will fly, then add a takeoff", emptyPlanText(false, false))
    }

    func testOnceTheMapIsSomewhereRealTheTakeoffButtonIsTheInstructionAgain() {
        XCTAssertEqual("Empty plan · add a takeoff to start", emptyPlanText(false, true))
    }

    func testWhereAWaypointMayBeAddedByHandTheLongPressStaysTheInstruction() {
        XCTAssertEqual("Empty plan · long press to add", emptyPlanText(true, false), "long press carries its own position, so it works wherever the camera is")
    }
}
