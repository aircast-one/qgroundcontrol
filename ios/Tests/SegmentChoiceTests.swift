import XCTest
@testable import Aircast

final class SegmentChoiceTests: XCTestCase {
    func testShortWritableEnumsBecomeSegments() {
        XCTAssertTrue(showsAsSegments(true, false, true, ["Indoor", "Outdoor", "System"]))
    }

    func testLongOrManyOptionsStayADropdown() {
        XCTAssertFalse(showsAsSegments(true, false, true, ["Feet/Second", "Meters/Second", "Miles/Hour", "Knots", "Km/Hour"]))
        XCTAssertFalse(showsAsSegments(true, false, true, ["A very long first option", "And a second one"]))
    }

    func testReadOnlyOrOffListValuesStayAsTheyAre() {
        XCTAssertFalse(showsAsSegments(true, false, false, ["On", "Off"]))
        XCTAssertFalse(showsAsSegments(true, true, true, ["On", "Off"]))
        XCTAssertFalse(showsAsSegments(false, false, true, []))
    }
}
