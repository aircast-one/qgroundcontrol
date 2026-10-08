import XCTest
@testable import Aircast

final class PipZoomTests: XCTestCase {
    func testTheSmallMapZoomsOutByThreeAndTheMainMapReturnsToItsOwnZoomLikeFlyViewMap() throws {
        XCTAssertEqual(try XCTUnwrap(pipZoom(15.0, true)), 12.0, accuracy: 0)
        XCTAssertNil(pipZoom(2.5, true), "QGC leaves zoom 3 and below alone")
        XCTAssertEqual(try XCTUnwrap(pipZoom(15.0, false)), 15.0, accuracy: 0)
        XCTAssertNil(pipZoom(0.0, false))
    }
}
