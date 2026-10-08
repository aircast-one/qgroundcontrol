import XCTest
@testable import Aircast

final class PinchZoomTests: XCTestCase {
    func testAPinchMapsToFlightDisplayViewVideosStepValues() {
        XCTAssertEqual(pinchStep(2.2), 2)
        XCTAssertEqual(pinchStep(1.1), 1)
        XCTAssertEqual(pinchStep(0.5), -5)
    }
}
