import UIKit
import XCTest
@testable import Aircast

final class DeviceCameraHostTests: XCTestCase {
    func testADeviceCameraIsTurnedUprightForEveryWayTheScreenCanFace() {
        XCTAssertEqual(deviceCameraRotation(90, front: false, displayDegrees: 0), 90)
        XCTAssertEqual(deviceCameraRotation(90, front: false, displayDegrees: 90), 0)
        XCTAssertEqual(deviceCameraRotation(90, front: false, displayDegrees: 270), 180)
        XCTAssertEqual(deviceCameraRotation(270, front: true, displayDegrees: 0), 270)
        XCTAssertEqual(deviceCameraRotation(270, front: true, displayDegrees: 90), 0)
        XCTAssertEqual(deviceCameraRotation(270, front: true, displayDegrees: 270), 180)
    }

    func testTheScreenTurnIsReadTheWayTheCoreCountsIt() {
        XCTAssertEqual(interfaceDegrees(.portrait), 0)
        XCTAssertEqual(interfaceDegrees(.landscapeRight), 90)
        XCTAssertEqual(interfaceDegrees(.portraitUpsideDown), 180)
        XCTAssertEqual(interfaceDegrees(.landscapeLeft), 270)
        XCTAssertEqual(deviceCameraTurn(0, displayDegrees: 0), 90)
        XCTAssertEqual(deviceCameraTurn(1, displayDegrees: 90), 0)
    }
}
