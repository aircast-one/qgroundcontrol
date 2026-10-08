import XCTest
@testable import Aircast

final class MotorsTests: XCTestCase {
    func testAVehicleThatNamesItsMotorCountIsBelieved() {
        XCTAssertEqual(motorCount(6), 6)
        XCTAssertNil(motorCountNotice(6))
    }

    func testAnAirframeWithNoPublishedLayoutGetsEightButtonsRatherThanNone() {
        XCTAssertEqual(motorCount(nil), 8)
    }

    func testAnUnknownCountWarnsWithMotorComponentsText() {
        XCTAssertEqual(motorCountNotice(nil), "Warning: Unable to determine motor count")
    }

    func testAnAbsentCountIsANullRatherThanANumberToBeSifted() {
        XCTAssertNil(
            reportedMotors(JSON.parse(#"{"connected":true,"motorCount":null}"#)),
            "view.frame already drops the -1 QGC answers for an airframe it has no layout for, and holds the count back for a submarine until its parameters arrive - a rule this head cannot express from a raw motorCount"
        )
        XCTAssertEqual(reportedMotors(JSON.parse(#"{"connected":true,"motorCount":4}"#)), 4)
        XCTAssertNil(reportedMotors(JSON.parse(#"{"connected":false}"#)))
        XCTAssertNil(reportedMotors(nil))
    }

    func testACountBelowOneIsNotALayout() {
        XCTAssertNil(reportedMotors(JSON.parse(#"{"motorCount":0}"#)))
    }
}
