import XCTest
@testable import Aircast

final class RecenterCameraTests: XCTestCase {
    private func gimbal(_ pitch: Double?, _ yaw: Double?) -> GimbalIndicatorState {
        GimbalIndicatorState(statusText: "", pitchText: "", yawText: "", yawLockLabel: "", yawLocked: false, yawLockOffered: false, retractOffered: false, controlOffered: false, controlLabel: "", haveControl: false, gimbals: [], pitchDegrees: pitch, yawDegrees: yaw)
    }

    func testRecenterShowsOnlyOnceTheCameraPointsAwayFromCentre() {
        XCTAssertNil(recenterTarget(syntheticAims: true, syntheticTilt: DEFAULT_TILT_DEG, syntheticPan: 0, gimbal: nil))
        XCTAssertEqual(recenterTarget(syntheticAims: true, syntheticTilt: DEFAULT_TILT_DEG, syntheticPan: 40, gimbal: nil), .Synthetic)
        XCTAssertNil(recenterTarget(syntheticAims: false, syntheticTilt: nil, syntheticPan: nil, gimbal: gimbal(0.5, -1)))
        XCTAssertEqual(recenterTarget(syntheticAims: false, syntheticTilt: nil, syntheticPan: nil, gimbal: gimbal(-45, 0)), .Gimbal)
    }

    func testAnAimableSyntheticViewIsRecentredRatherThanTheGimbalBehindIt() {
        XCTAssertNil(recenterTarget(syntheticAims: true, syntheticTilt: DEFAULT_TILT_DEG, syntheticPan: 0, gimbal: gimbal(-45, 0)))
    }

    func testTheReadoutRoundsToWholeDegrees() {
        XCTAssertEqual(tiltText(-11.4), "-11\u{00B0}")
        XCTAssertEqual(tiltText(0.2), "0\u{00B0}")
    }

    func testNothingToRecenterWithoutAnAimableViewOrAGimbal() {
        XCTAssertNil(recenterTarget(syntheticAims: false, syntheticTilt: -60, syntheticPan: 90, gimbal: nil))
    }

    func testTheGimbalReadsItsYawFromTheBodyFrame() {
        let view = JSON.parse(#"{"shown":true,"pitchDegrees":-20,"bodyYawDegrees":35,"gimbals":[]}"#)
        XCTAssertEqual(gimbalIndicator(view)?.yawDegrees, 35)
    }
}
