import MapLibre
import XCTest
@testable import Aircast

final class VehicleHeadingTests: XCTestCase {
    func testAKnownHeadingRidesAlongOnTheFeature() throws {
        let feature = vehicleFeature(41.0, 44.0, 90.0)
        XCTAssertTrue(feature.hasProperty(HEADING_PROPERTY))
        XCTAssertEqual(try XCTUnwrap(feature.getNumberProperty(HEADING_PROPERTY)), 90.0, accuracy: 1e-9)
    }

    func testAnUnknownHeadingLeavesThePropertyOffSoTheArrowIsFilteredOut() {
        XCTAssertFalse(vehicleFeature(41.0, 44.0, .nan).hasProperty(HEADING_PROPERTY))
    }

    func testHeadingsAreNormalisedIntoASingleTurn() throws {
        let wrapped = vehicleFeature(41.0, 44.0, 450.0)
        let negative = vehicleFeature(41.0, 44.0, -90.0)
        XCTAssertEqual(try XCTUnwrap(wrapped.getNumberProperty(HEADING_PROPERTY)), 90.0, accuracy: 1e-9)
        XCTAssertEqual(try XCTUnwrap(negative.getNumberProperty(HEADING_PROPERTY)), 270.0, accuracy: 1e-9)
    }

    func testThePositionStillRidesAlongWithoutAHeading() throws {
        let point = try XCTUnwrap(vehicleFeature(41.5, 44.5, .nan) as? MLNPointFeature)
        XCTAssertEqual(point.coordinate.latitude, 41.5, accuracy: 1e-9)
        XCTAssertEqual(point.coordinate.longitude, 44.5, accuracy: 1e-9)
    }
}
