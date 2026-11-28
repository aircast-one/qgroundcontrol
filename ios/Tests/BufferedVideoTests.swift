import XCTest
@testable import Aircast

final class BufferedVideoTests: XCTestCase {
    private let twin = "http://falcon:8888/front/index.m3u8"

    private func view(_ endpoints: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Video","hlsEndpoints":\#(endpoints)}"#)
    }

    func testTheCoreNamesTheBufferedTwinPerChannelAndNothingForACameraOnWebRTC() {
        let fallen = view(#"[null,"\#(twin)"]"#)
        XCTAssertNil(hlsEndpoint(fallen, MAIN_VIDEO_CHANNEL))
        XCTAssertEqual(hlsEndpoint(fallen, PIP_VIDEO_CHANNEL), twin)
        XCTAssertNil(hlsEndpoint(view(#"[""]"#), MAIN_VIDEO_CHANNEL))
        XCTAssertNil(hlsEndpoint(nil, MAIN_VIDEO_CHANNEL))
    }

    func testABufferedTwinIsPlayedInPlaceOfThePipelineAndThePipelineComesBackWhenItGoes() {
        XCTAssertEqual(wantedPlayback(pipeline: "whepsrc ! appsink", buffered: nil), .pipeline("whepsrc ! appsink"))
        XCTAssertEqual(wantedPlayback(pipeline: nil, buffered: twin), .buffered(twin))
        XCTAssertEqual(wantedPlayback(pipeline: "whepsrc ! appsink", buffered: twin), .buffered(twin), "the two never run side by side")
        XCTAssertNil(wantedPlayback(pipeline: nil, buffered: nil))
    }

    func testTheReportCountsOnlyWhileThePictureMovesSoTheCoreCanTellAStallOrATwinThatNeverPlayed() {
        let size = CGSize(width: 1280, height: 720)
        XCTAssertEqual(
            bufferedReading(ready: false, seconds: 0, size: .zero, error: "The requested URL was not found on this server."),
            BufferedReading(running: false, frames: 0, width: 0, height: 0, error: "The requested URL was not found on this server."),
            "a device with no HLS twin reports no picture, which is what sends the core back to WebRTC")
        XCTAssertEqual(bufferedReading(ready: true, seconds: .nan, size: size, error: nil).frames, 0, "a live stream that has not started has no time yet")
        let playing = bufferedReading(ready: true, seconds: 2, size: size, error: nil)
        XCTAssertEqual(playing, BufferedReading(running: true, frames: 60, width: 1280, height: 720, error: ""))
        XCTAssertEqual(bufferedReading(ready: true, seconds: 2, size: size, error: nil).frames, playing.frames, "a frozen picture stops counting")
        XCTAssertGreaterThan(bufferedReading(ready: true, seconds: 2.5, size: size, error: nil).frames, playing.frames)
    }

    func testTheNoticeShowsOnlyForAChannelWhoseBufferedTwinIsPlaying() {
        let fallen = view(#"[null,"\#(twin)"]"#)
        XCTAssertTrue(bufferedNoteShown(fallen, PIP_VIDEO_CHANNEL, playing: true))
        XCTAssertFalse(bufferedNoteShown(fallen, PIP_VIDEO_CHANNEL, playing: false), "nothing is said over a twin that shows no picture yet")
        XCTAssertFalse(bufferedNoteShown(fallen, MAIN_VIDEO_CHANNEL, playing: true))
    }
}
