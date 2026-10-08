import XCTest
@testable import Aircast

final class VehicleRowCompassTests: XCTestCase {
    func testADisarmedVehiclesCompassIsDimmedLikeMultiVehicleLists() {
        XCTAssertEqual(rowCompassAlpha(true), 1)
        XCTAssertEqual(rowCompassAlpha(false), 0.5)
    }
}
