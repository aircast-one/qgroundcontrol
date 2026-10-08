import XCTest
@testable import Aircast

final class CameraControlLayerTests: XCTestCase {
    func testTheStreamShutterIsAVideoShutterThatSaysWhatATapWillDo() {
        let idle = streamShutter(false, true, nil)
        XCTAssertEqual([idle.label, "\(idle.recording)", "\(idle.video)"], ["Start recording", "false", "true"])
        let recording = streamShutter(true, true, 5)
        XCTAssertEqual([recording.label, "\(recording.recording)", "\(recording.video)"], ["Stop recording", "true", "true"])
    }

    func testAStreamThatCannotRecordShowsADeadShutterButOneAlreadyRecordingCanStillStop() {
        XCTAssertFalse(streamShutter(false, false, nil).enabled)
        XCTAssertTrue(streamShutter(true, false, 3).enabled)
    }

    func testTheStreamRecordingClockReadsLikeTheCameras() {
        XCTAssertEqual(shutterReadout(streamShutter(true, true, 65)), "REC 00:01:05")
        XCTAssertNil(shutterReadout(streamShutter(false, true, nil)))
        XCTAssertEqual(recordClock(3600), "01:00:00")
    }
}
