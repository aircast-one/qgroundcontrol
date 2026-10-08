import XCTest
@testable import Aircast

final class CentreInsetTests: XCTestCase {
    func testTheMapOnlyRecentresOnceTheVehicleLeavesTheMiddleOfTheScreen() {
        XCTAssertFalse(outsideCentreInset(500, 500, 1000, 1000, 0, 0))
        XCTAssertTrue(outsideCentreInset(100, 500, 1000, 1000, 0, 0), "near the left edge")
        XCTAssertTrue(outsideCentreInset(500, 700, 1000, 1000, 0, 200), "behind the bottom controls")
        XCTAssertFalse(outsideCentreInset(10, 10, 0, 0, 0, 0), "an unmeasured map never asks to move")
    }

    func testTheVehicleSitsInTheMiddleOfTheClearArea() {
        XCTAssertEqual(clearAreaLift(300, 100), 100, "chrome 300 px on top and 100 px below puts the vehicle 100 px under the map centre")
        XCTAssertEqual(clearAreaLift(0, 0), 0)
    }
}
