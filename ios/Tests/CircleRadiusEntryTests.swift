import XCTest
@testable import Aircast

final class CircleRadiusEntryTests: XCTestCase {
    private let circle = FenceCircle(index: 0, inclusion: true, centre: TrackPoint(47.0, 8.0), radius: 50.0, radiusMinimum: 10.0, radiusMaximum: 500.0)

    func testATypedRadiusIsTakenOnlyInsideTheFencesBounds() throws {
        XCTAssertEqual(120.5, try XCTUnwrap(typedRadius(" 120.5 ", circle)), accuracy: 0)
        XCTAssertNil(typedRadius("5", circle))
        XCTAssertNil(typedRadius("900", circle))
        XCTAssertNil(typedRadius("wide", circle))
        XCTAssertEqual("120.5", trimmedRadius(120.5))
        XCTAssertEqual("50", trimmedRadius(50.0))
    }
}
