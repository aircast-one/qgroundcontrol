import XCTest
@testable import Aircast

final class GamepadStreamsTests: XCTestCase {
    func testTheGamepadStreamsOnlyWhileItDrivesAVehicleOrIsBeingCalibrated() {
        XCTAssertTrue(streams(JSON.parse(#"{"vehicle":true,"enabled":true,"calibration":null}"#)))
        XCTAssertFalse(streams(JSON.parse(#"{"vehicle":true,"enabled":false,"calibration":null}"#)))
        XCTAssertFalse(streams(JSON.parse(#"{"vehicle":false,"enabled":true,"calibration":null}"#)))
        XCTAssertTrue(streams(JSON.parse(#"{"vehicle":false,"enabled":false,"calibration":{"step":1}}"#)))
        XCTAssertFalse(streams(nil))
    }
}
