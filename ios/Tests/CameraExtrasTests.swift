import XCTest
@testable import Aircast

final class CameraExtrasTests: XCTestCase {
    func testTheCameraSectionsIntervalModeAndGimbalComeFromTheCore() throws {
        let extras = try XCTUnwrap(cameraExtras(JSON.parse(#"{"available":true,"intervalTime":{"value":4.0},"intervalDistance":null,"cameraModeSupported":true,"commandsMode":true,"cameraMode":{"choice":1},"commandsGimbal":true,"gimbalPitch":{"value":-45.0},"gimbalYaw":null}"#)))
        XCTAssertEqual(4.0, try XCTUnwrap(extras.intervalTime), accuracy: 0)
        XCTAssertNil(extras.intervalDistance)
        XCTAssertEqual(1, extras.mode)
        XCTAssertEqual(-45.0, extras.pitch, accuracy: 0)
        XCTAssertEqual(0.0, extras.yaw, accuracy: 0)
        XCTAssertNil(cameraExtras(JSON.parse(#"{"available":false}"#)))
        XCTAssertEqual("10", trimmedNumber(10.0))
    }

    func testTheGimbalAnglesSlideOverTheFactsUserRangeLikeCameraSectionsFactTextFieldSlider() throws {
        let extras = try XCTUnwrap(cameraExtras(JSON.parse(#"{"available":true,"commandsGimbal":true,"gimbalPitch":{"value":45.0,"slider":{"from":90.0,"to":0.0,"decimals":0}},"gimbalYaw":{"value":0.0,"slider":{"from":-180.0,"to":180.0,"decimals":0}}}"#)))
        XCTAssertEqual(0.0...90.0, extras.pitchRange)
        XCTAssertEqual(-180.0...180.0, extras.yawRange)
        XCTAssertNil(try XCTUnwrap(cameraExtras(JSON.parse(#"{"available":true,"gimbalPitch":{"value":-45.0}}"#))).pitchRange)
    }

    func testThePhotoDistanceIsLabelledInTheUnitsItsFactIsCookedTo() throws {
        let feet = try XCTUnwrap(cameraExtras(JSON.parse(#"{"available":true,"intervalDistance":{"value":100.0,"units":"ft"}}"#)))
        XCTAssertEqual(100.0, try XCTUnwrap(feet.intervalDistance), accuracy: 0)
        XCTAssertEqual("ft", feet.distanceUnits)
        XCTAssertEqual("m", try XCTUnwrap(cameraExtras(JSON.parse(#"{"available":true,"intervalDistance":{"value":3.0}}"#))).distanceUnits)
    }
}
