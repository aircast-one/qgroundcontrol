import XCTest
@testable import Aircast

final class WaypointSpeedTests: XCTestCase {
    func testTheSpeedRowReadsTheItemsSpeedSectionAndSkipsAVehicleThatHasNone() {
        let served = JSON.parse(#"{"speedSection":{"available":true,"specified":true,"value":8.0,"units":"m/s","path":"p","specifyPath":"s"}}"#)
        XCTAssertEqual(WaypointSpeed(specified: true, value: 8.0, units: "m/s", path: "p", specifyPath: "s"), waypointSpeed(served))
        XCTAssertNil(waypointSpeed(JSON.parse(#"{"speedSection":{"available":false}}"#)))
    }

    func testAWaypointsHoldIsReadInSecondsAndAnItemWithoutOneShowsNoRow() {
        XCTAssertEqual(WaypointHold(seconds: 5.0, units: "s", path: "p"), waypointHold(JSON.parse(#"{"hold":{"value":5.0,"units":"s","path":"p"}}"#)))
        XCTAssertNil(waypointHold(JSON.parse(#"{"hold":null}"#)))
    }
}
