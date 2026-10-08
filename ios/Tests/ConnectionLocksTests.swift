import XCTest
@testable import Aircast

final class ConnectionLocksTests: XCTestCase {
    func testTheLocksFollowMultiVehicleManagersFirstVehicleInLastVehicleOut() {
        XCTAssertTrue(vehicleConnected(JSON.parse(#"{"count":1}"#)))
        XCTAssertTrue(vehicleConnected(JSON.parse(#"{"count":3}"#)))
        XCTAssertFalse(vehicleConnected(JSON.parse(#"{"count":0}"#)))
        XCTAssertFalse(vehicleConnected(nil))
    }
}
