import XCTest
@testable import Aircast

final class PlanItemCountTests: XCTestCase {
    private func model(_ count: Int) -> JSON {
        JSON.parse(#"{"kind":"object","items":["# + Array(repeating: "{}", count: count).joined(separator: ",") + "]}")
    }

    func testTheSettingsItemIsNotSomethingThePilotAdded() {
        XCTAssertEqual(0, planItemCount(model(1)))
        XCTAssertEqual(3, planItemCount(model(4)))
    }

    func testAPlanThatCouldNotBeReadCountsNothingRatherThanGoingNegative() {
        XCTAssertEqual(0, planItemCount(nil))
        XCTAssertEqual(0, planItemCount(model(0)))
    }

    func testATakeoffAndAReturnToLaunchAreNamedByFlagAndCommand() {
        let plan = JSON.parse(#"{"kind":"object","items":[{},{"kind":"takeoff","name":"Start"},{"kind":"command","command":20,"endsRoute":true,"name":"irrelevant"},{"kind":"waypoint","flownLeg":true}]}"#)
        XCTAssertEqual(["takeoff", "RTL", "1 after RTL"], planShape(plan))
    }

    func testAPlanWithNeitherNamesNothing() {
        XCTAssertEqual([], planShape(model(3)))
        XCTAssertEqual([], planShape(nil))
    }

    func testAnItemAddedAfterTheLandingIsNamedAsSuch() {
        let plan = JSON.parse(#"{"kind":"object","items":[{},{"kind":"waypoint","flownLeg":true},{"kind":"command","command":20,"endsRoute":true},{"kind":"waypoint","flownLeg":true},{"kind":"waypoint","flownLeg":true}]}"#)
        XCTAssertEqual(["RTL", "2 after RTL"], planShape(plan))
    }

    func testAPlanThatEndsAtItsLandingStrandsNothing() {
        let plan = JSON.parse(#"{"kind":"object","items":[{},{"kind":"waypoint","flownLeg":true},{"kind":"command","command":20,"endsRoute":true}]}"#)
        XCTAssertEqual(["RTL"], planShape(plan))
    }
}
