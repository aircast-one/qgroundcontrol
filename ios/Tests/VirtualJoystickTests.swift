import XCTest
@testable import Aircast

final class VirtualJoystickTests: XCTestCase {
    private let shown = VirtualJoystickState(show: true, sending: true, autoCenterThrottle: false, leftHandedMode: false, leftPositiveOnly: true, rightPositiveOnly: false, periodMs: 40)

    private func changed(_ change: (inout VirtualJoystickState) -> Void) -> VirtualJoystickState {
        var next = shown
        change(&next)
        return next
    }

    func testThePadMapsItsPositionTheWayJoystickThumbPadDoes() {
        XCTAssertEqual(stickAxes(0.5, 0.5, false), StickAxes(x: 0.0, y: 0.0))
        XCTAssertEqual(stickAxes(0, 0, false), StickAxes(x: -1.0, y: 1.0))
        XCTAssertEqual(stickAxes(1, 0.5, true), StickAxes(x: 1.0, y: 0.5))
        XCTAssertEqual(stickAxes(0.5, 1, true), StickAxes(x: 0.0, y: 0.0))
        XCTAssertEqual(restingY(false), 1)
        XCTAssertEqual(restingY(true), 0.5)
    }

    func testTheRightStickFliesRollAndPitchUnlessTheLayoutIsLeftHanded() {
        let throttle = StickAxes(x: 0.1, y: 0.9)
        let attitude = StickAxes(x: 0.3, y: -0.4)
        XCTAssertEqual(joystickValues(throttle, attitude, false), [0.3, -0.4, 0.1, 0.9])
        XCTAssertEqual(joystickValues(throttle, attitude, true), [0.1, 0.9, 0.3, -0.4])
    }

    func testWithTheSticksNeverTouchedTheRestingPositionsAreSentAsVirtualJoystickSendsFromTheStart() {
        XCTAssertEqual(stickValues(shown, nil, nil), joystickValues(stickAxes(0.5, 1, true), stickAxes(0.5, 0.5, false), false))
        let held = CGPoint(x: 0.2, y: 0.3)
        XCTAssertEqual(
            stickValues(changed { $0.leftHandedMode = true }, held, nil),
            joystickValues(stickAxes(0.2, 0.3, true), stickAxes(0.5, 0.5, false), true),
            "values follow the current layout, not the one in force when the stick was held"
        )
        XCTAssertEqual(released(held, false), CGPoint(x: 0.5, y: 0.3), "a released throttle stays where it was without auto-centre")
    }

    func testHeldSticksAreDroppedWhenTheJoystickHidesOrThrottleAutoCentreFlipsAndKeptOtherwise() {
        XCTAssertTrue(sticksReset(shown, changed { $0.show = false }))
        XCTAssertTrue(sticksReset(shown, changed { $0.autoCenterThrottle = true }))
        XCTAssertTrue(sticksReset(nil, nil))
        XCTAssertFalse(sticksReset(shown, changed { $0.sending = false }))
        XCTAssertFalse(sticksReset(shown, changed { $0.leftHandedMode = true }))
    }
}
