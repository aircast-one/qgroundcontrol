import XCTest
@testable import Aircast

final class CameraSwitchTests: XCTestCase {
    private func camera(_ slot: Int, active: Bool = false, problem: String? = nil) -> String {
        #"{"slot":\#(slot),"title":"Camera \#(slot + 1)","short":"Cam \#(slot + 1)","active":\#(active),"status":"live","problem":\#(problem.map { "\"\($0)\"" } ?? "null")}"#
    }

    private func reading(_ cameras: String..., pip: String = #"{"enabled":false,"slot":null}"#) -> CamerasReading? {
        camerasReading(JSON.parse(#"{"class":"Cameras","pip":\#(pip),"cameras":[\#(cameras.joined(separator: ","))]}"#))
    }

    func testThePickerHidesUntilThereAreTwoCamerasToChooseBetween() {
        XCTAssertNil(cameraSwitchState(nil))
        XCTAssertNil(cameraSwitchState(reading(camera(0, active: true))))
        XCTAssertNil(cameraSwitchState(reading(camera(0, active: true), camera(1, problem: "No address"))))
    }

    func testThePickerListsEveryCameraThatCanPlayTheOneOnScreenChosen() {
        let state = cameraSwitchState(reading(camera(0, active: true, problem: "No address"), camera(1), camera(2, problem: "No address"), camera(3)))!
        XCTAssertEqual(state.cameras.map(\.slot), [0, 1, 3])
        XCTAssertEqual(state.shown.slot, 0)
    }

    func testTheSecondCameraShowsOnlyWhenItIsSwitchedOnAndThereIsOne() {
        XCTAssertEqual(pipCamera(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":1}"#), true)?.slot, 1)
        XCTAssertNil(pipCamera(reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#), true))
        XCTAssertNil(pipCamera(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":null}"#), true))
    }

    func testTheSplitButtonFlipsTheSettingAndHidesWithNoSecondCamera() {
        XCTAssertEqual(splitToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#), true, true), true)
        XCTAssertEqual(splitToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":1}"#), true, true), false)
        XCTAssertNil(splitToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":null}"#), true, true))
        XCTAssertNil(splitToggleTarget(nil, true, true))
    }

    func testTheSplitNeedsTheVideoOnScreen() {
        XCTAssertNil(splitToggleTarget(reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#), false, true))
    }

    func testWithVideoSwitchedOffNeitherTheSecondCameraNorItsButtonShowsEvenBeforeTheCoreCatchesUp() {
        let stale = reading(camera(0, active: true), camera(1), pip: #"{"enabled":true,"slot":1}"#)
        XCTAssertNil(pipCamera(stale, false))
        XCTAssertNil(splitToggleTarget(stale, true, false))
        let off = reading(camera(0, active: true), camera(1), pip: #"{"enabled":false,"slot":1}"#)
        XCTAssertNil(splitToggleTarget(off, true, false), "the button never offers a switch-on that cannot show anything")
    }

    func testWithNoCameraOnScreenTheSwitchShowsTheFirstOne() {
        let state = cameraSwitchState(reading(camera(0), camera(1)))!
        XCTAssertEqual(state.shown.slot, 0)
    }

    func testASwipeStepsToTheNeighbouringCameraAndWrapsAroundTheEnds() {
        let state = cameraSwitchState(reading(camera(0), camera(1, active: true), camera(2)))
        XCTAssertEqual(neighbourCamera(state, 1)?.slot, 2)
        XCTAssertEqual(neighbourCamera(state, -1)?.slot, 0)
        XCTAssertEqual(neighbourCamera(cameraSwitchState(reading(camera(0), camera(1), camera(2, active: true))), 1)?.slot, 0)
        XCTAssertNil(neighbourCamera(cameraSwitchState(reading(camera(0, active: true))), 1), "one camera has nowhere to swipe to")
    }

    func testTheChipNamesTheCameraOnScreenOrTheFirstOneWhenNoneIs() {
        XCTAssertEqual(chipCamera(reading(camera(0), camera(1, active: true)))?.slot, 1)
        XCTAssertEqual(chipCamera(reading(camera(0), camera(1)))?.slot, 0)
        XCTAssertNil(chipCamera(reading()))
        XCTAssertNil(chipCamera(nil))
    }

    func testThePilotsPickShowsAtOnceAndHoldsUntilTheCoreSendsACameraListMadeAfterIt() {
        let shown = JSON.parse(#"{"class":"Cameras","cameras":[\#(camera(0, active: true)),\#(camera(1))]}"#)
        XCTAssertEqual(pickedReading(shown, CameraPick(slot: 1, on: shown))?.cameras.map(\.active), [false, true], "the pick moves the active camera before the core answers")
        XCTAssertEqual(pickedReading(shown, nil)?.cameras.map(\.active), [true, false])
        let answered = JSON.parse(#"{"class":"Cameras","cameras":[\#(camera(0, active: true)),\#(camera(2))]}"#)
        XCTAssertEqual(pickedReading(answered, CameraPick(slot: 1, on: shown))?.cameras.map(\.active), [true, false], "a list the core sent after the pick is the core's word")
        XCTAssertNil(pickedReading(nil, CameraPick(slot: 1, on: nil)))
    }

    func testTheSyntheticCameraCountsAsChosenOnlyWhenItIsTheOneOnScreen() {
        let synthetic = #"{"slot":1,"title":"Synthetic","short":"Synthetic","source":"Synthetic View","active":true,"status":"idle","problem":null}"#
        XCTAssertTrue(syntheticChosen(reading(camera(0), synthetic)))
        XCTAssertFalse(syntheticChosen(reading(camera(0, active: true), synthetic.replacingOccurrences(of: #""active":true"#, with: #""active":false"#))))
        XCTAssertFalse(syntheticChosen(nil))
    }
}
