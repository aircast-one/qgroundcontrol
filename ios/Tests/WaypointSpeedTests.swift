import XCTest
@testable import Aircast

final class WaypointSpeedTests: XCTestCase {
    func testABlankFieldClearsTheSpeedAPositiveNumberSetsItAnythingElseIsRefused() {
        XCTAssertEqual(SpeedEntry.Clear, speedEntry("  "))
        XCTAssertEqual(SpeedEntry.Set(value: 8.5), speedEntry("8,5"))
        XCTAssertEqual(SpeedEntry.Invalid, speedEntry("0"))
        XCTAssertEqual(SpeedEntry.Invalid, speedEntry("fast"))
    }

    func testTheFieldShowsTheItemsSpeedOnlyWhenTheItemSpecifiesOne() throws {
        let speed = try XCTUnwrap(waypointSpeed(JSON.parse(#"{"speedSection":{"available":true,"specified":true,"value":8.0,"units":"m/s","path":"p","specifyPath":"s"}}"#)))
        XCTAssertEqual("8", speedFieldText(speed))
        var unspecified = speed
        unspecified.specified = false
        XCTAssertEqual("", speedFieldText(unspecified))
        XCTAssertNil(waypointSpeed(JSON.parse(#"{"speedSection":{"available":false}}"#)))
    }

    func testAWaypointsHoldIsSecondsFromZeroAnEmptyFieldMeaningNoHold() throws {
        XCTAssertEqual(WaypointHold(seconds: 5.0, units: "s", path: "p"), waypointHold(JSON.parse(#"{"hold":{"value":5.0,"units":"s","path":"p"}}"#)))
        XCTAssertNil(waypointHold(JSON.parse(#"{"hold":null}"#)))
        XCTAssertEqual(0.0, try XCTUnwrap(holdEntry(" ")), accuracy: 0)
        XCTAssertEqual(2.5, try XCTUnwrap(holdEntry("2,5")), accuracy: 0)
        XCTAssertNil(holdEntry("-1"))
        XCTAssertNil(holdEntry("long"))
    }
}
