import XCTest
@testable import Aircast

final class GroundOutlineTests: XCTestCase {
    func testTheFillClosesUnderTheTerrainItHasNotAcrossTheWholeChart() {
        XCTAssertEqual(
            [CGPoint(x: 10, y: 5), CGPoint(x: 40, y: 8), CGPoint(x: 40, y: 100), CGPoint(x: 10, y: 100)],
            groundOutline([CGPoint(x: 10, y: 5), CGPoint(x: 40, y: 8)], 100)
        )
    }

    func testTerrainCoveringTheWholeWidthStillClosesAtTheCorners() {
        XCTAssertEqual(
            [CGPoint(x: 0, y: 5), CGPoint(x: 200, y: 8), CGPoint(x: 200, y: 100), CGPoint(x: 0, y: 100)],
            groundOutline([CGPoint(x: 0, y: 5), CGPoint(x: 200, y: 8)], 100)
        )
    }

    func testASingleSampleIsNotAShape() {
        XCTAssertEqual([], groundOutline([CGPoint(x: 10, y: 5)], 100))
    }

    func testNoTerrainDrawsNoGround() {
        XCTAssertEqual([], groundOutline([], 100))
    }
}
