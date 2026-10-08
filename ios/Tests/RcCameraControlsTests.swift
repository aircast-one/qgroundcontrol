import XCTest
@testable import Aircast

final class RcCameraControlsTests: XCTestCase {
    func testAPanChannelSharedWithTiltIsDroppedAsCameraControlLayerReadsIt() {
        XCTAssertEqual(rcCameraChannels(tilt: 7, pan: 7, zoom: 0, light: 0, record: 0).pan, 0)
        XCTAssertEqual(rcCameraChannels(tilt: 7, pan: 8, zoom: 0, light: 0, record: 0).pan, 8)
        XCTAssertFalse(rcCameraChannels(tilt: 0, pan: 0, zoom: 0, light: 0, record: 0).any)
        XCTAssertTrue(rcCameraChannels(tilt: 0, pan: 0, zoom: 9, light: 0, record: 0).any)
    }

    func testRecordShowsOnWhenEitherTheStreamOrTheRecordChannelRecordsAsRecordingDoes() {
        XCTAssertTrue(cameraRecording(recordChannel: 0, channelRecording: false, streamRecording: true))
        XCTAssertTrue(cameraRecording(recordChannel: 5, channelRecording: true, streamRecording: false))
        XCTAssertFalse(cameraRecording(recordChannel: 0, channelRecording: true, streamRecording: false))
    }
}
