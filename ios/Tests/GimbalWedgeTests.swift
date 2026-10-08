import XCTest
@testable import Aircast

final class GimbalWedgeTests: XCTestCase {
    func testEachGimbalsYawIsReadForTheMapWedge() {
        let read = gimbalAzimuths(JSON.parse(#"{"gimbals":[{"yaw":270.0,"active":true},{"yaw":10.0,"active":false}]}"#))
        XCTAssertEqual(read, [GimbalAzimuth(yaw: 270.0, active: true), GimbalAzimuth(yaw: 10.0, active: false)])
        XCTAssertEqual(gimbalAzimuths(nil), [])
    }
}
