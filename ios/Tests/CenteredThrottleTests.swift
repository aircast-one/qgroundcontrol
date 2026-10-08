import XCTest
@testable import Aircast

final class CenteredThrottleTests: XCTestCase {
    func testTheRadioViewCarriesCenteredThrottleAndJoystickMode() throws {
        let read = try XCTUnwrap(radioView(JSON.parse(#"{"class":"Radio","connected":true,"centeredThrottle":true,"joystickMode":false}"#)))
        XCTAssertTrue(read.centeredThrottle)
        XCTAssertFalse(read.joystickMode)
    }
}
