import XCTest
@testable import Aircast

final class OptionalFactTests: XCTestCase {
    func testAnOptionalFieldReadsSwitchedOffWhileItHoldsNoNumber() {
        let off = factFromControl(JSON.parse(#"{"name":"Yaw","path":"p","optional":true,"value":null,"valueText":""}"#))!
        XCTAssertTrue(off.optional)
        XCTAssertFalse(off.optionalSet)
        let on = factFromControl(JSON.parse(#"{"name":"Yaw","path":"p","optional":true,"value":15.0,"valueText":"15.0"}"#))!
        XCTAssertTrue(on.optionalSet)
    }
}
