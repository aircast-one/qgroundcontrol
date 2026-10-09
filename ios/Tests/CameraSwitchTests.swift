import XCTest
@testable import Aircast

final class CameraSwitchTests: XCTestCase {
    private func camera(_ slot: Int, active: Bool = false, problem: String? = nil) -> String {
        #"{"slot":\#(slot),"title":"Camera \#(slot + 1)","short":"Cam \#(slot + 1)","active":\#(active),"status":"live","problem":\#(problem.map { "\"\($0)\"" } ?? "null")}"#
    }

    private func reading(_ cameras: String..., pip: String = #"{"enabled":false,"slot":null}"#) -> CamerasReading? {
        camerasReading(JSON.parse(#"{"class":"Cameras","pip":\#(pip),"cameras":[\#(cameras.joined(separator: ","))]}"#))
    }

    func testTheSwitchHidesUntilThereAreTwoCamerasToSwitchBetween() {
        XCTAssertNil(cameraSwitchState(nil))
        XCTAssertNil(cameraSwitchState(reading(camera(0, active: true))))
        XCTAssertNil(cameraSwitchState(reading(camera(0, active: true), camera(1, problem: "No address"))))
    }

    func testWithTwoCamerasATapShowsTheOtherOne() {
        let state = cameraSwitchState(reading(camera(0), camera(1, active: true)))!
        XCTAssertEqual(state.shown.slot, 1)
        XCTAssertEqual(state.toggleTo, 0)
    }

    func testWithMoreCamerasATapOpensTheListWhichLeavesOutTheOnesThatCannotPlay() {
        let state = cameraSwitchState(reading(camera(0, active: true, problem: "No address"), camera(1), camera(2, problem: "No address"), camera(3)))!
        XCTAssertNil(state.toggleTo)
        XCTAssertEqual(state.cameras.map(\.slot), [0, 1, 3])
        XCTAssertEqual(state.shown.slot, 0)
    }

    func testTheSecondCameraShowsOnlyWhenItIsSwitchedOnAndThereIsOne() {
        XCTAssertEqual(pipCamera(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":1}"#), true)?.slot, 1)
        XCTAssertNil(pipCamera(reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#), true))
        XCTAssertNil(pipCamera(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":null}"#), true))
    }

    func testThePictureInPictureButtonFlipsTheSettingAndHidesWithNoSecondCamera() {
        XCTAssertEqual(pipToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#), true, true), true)
        XCTAssertEqual(pipToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":1}"#), true, true), false)
        XCTAssertNil(pipToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":null}"#), true, true))
        XCTAssertNil(pipToggleTarget(nil, true, true))
    }

    func testThePictureInPictureButtonHidesWhereTheViewHasNoRoomForTheThumbnail() {
        XCTAssertNil(pipToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#), false, true))
    }

    func testWithVideoSwitchedOffNeitherTheSecondCameraNorItsButtonShowsEvenBeforeTheCoreCatchesUp() {
        let stale = reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":1}"#)
        XCTAssertNil(pipCamera(stale, false))
        XCTAssertNil(pipToggleTarget(stale, true, false))
        let off = reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#)
        XCTAssertNil(pipToggleTarget(off, true, false), "the button never offers a switch-on that cannot show anything")
    }

    func testADoubleTapOnTheTwoCameraSwitchSendsOneSwitch() {
        let first = SwitchTap(from: 0, atMs: 1_000)
        XCTAssertTrue(switchTapAllowed(nil, 0, 1_000))
        XCTAssertFalse(switchTapAllowed(first, 0, 1_150), "the second tap of a double tap, before the core answers")
        XCTAssertFalse(switchTapAllowed(first, 1, 1_200), "the second tap of a double tap, after the core already switched")
        XCTAssertFalse(switchTapAllowed(first, 0, 1_900), "a slow repeat while the switch is still pending")
        XCTAssertTrue(switchTapAllowed(first, 1, 1_400), "once the camera changed, the next tap switches back")
        XCTAssertTrue(switchTapAllowed(first, 0, 3_000), "a switch the core never answered stops blocking")
    }

    func testWithNoCameraOnScreenTheSwitchShowsTheFirstOne() {
        let state = cameraSwitchState(reading(camera(0), camera(1)))!
        XCTAssertEqual(state.shown.slot, 0)
        XCTAssertEqual(state.toggleTo, 1)
    }

    func testASwipeStepsToTheNeighbouringCameraAndWrapsAroundTheEnds() {
        let state = cameraSwitchState(reading(camera(0), camera(1, active: true), camera(2)))
        XCTAssertEqual(neighbourCamera(state, 1)?.slot, 2)
        XCTAssertEqual(neighbourCamera(state, -1)?.slot, 0)
        XCTAssertEqual(neighbourCamera(cameraSwitchState(reading(camera(0), camera(1), camera(2, active: true))), 1)?.slot, 0)
        XCTAssertNil(neighbourCamera(cameraSwitchState(reading(camera(0, active: true))), 1), "one camera has nowhere to swipe to")
    }
}
