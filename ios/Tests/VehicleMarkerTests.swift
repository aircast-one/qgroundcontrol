import XCTest
@testable import Aircast

final class VehicleMarkerTests: XCTestCase {
    func testAVehicleWithAPositionIsDrawn() throws {
        let features = vehicleFeatures(-35.36, 149.16, 90.0).shapes
        XCTAssertEqual(features.count, 1)
        XCTAssertEqual(try XCTUnwrap(features.first?.getNumberProperty(HEADING_PROPERTY)), 90.0, accuracy: 1e-9)
    }

    func testAVehicleThatHasGoneAwayIsNotLeftOnTheMap() {
        XCTAssertTrue(vehicleFeatures(.nan, .nan, 90.0).shapes.isEmpty)
        XCTAssertTrue(vehicleFeatures(0.0, 0.0, 90.0).shapes.isEmpty)
    }

    func testAPositionWithoutHeadingStillDrawsTheDot() {
        let features = vehicleFeatures(-35.36, 149.16, .nan).shapes
        XCTAssertEqual(features.count, 1)
        XCTAssertFalse(features[0].hasProperty(HEADING_PROPERTY))
    }
}
