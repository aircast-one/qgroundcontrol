import XCTest
@testable import Aircast

final class VideoViewTests: XCTestCase {
    private func view(_ active: Int, _ statuses: String..., extra: String = "") -> JSON {
        let cameras = statuses.enumerated().map { slot, status in
            #"{"slot":\#(slot),"title":"Camera \#(slot + 1)","status":"\#(status)","connecting":false,"recording":false,"configured":\#(!status.isBlank && status != "No stream URL")}"#
        }.joined(separator: ",")
        return JSON.parse(#"{"kind":"object","class":"Video","available":true,"decoding":false,"summary":"Not streaming.","activeSource":\#(active),"cameras":[\#(cameras)]\#(extra)}"#)
    }

    func testTheReadingCarriesTheCoresSummaryAndCameras() throws {
        let reading = try XCTUnwrap(videoReading(view(1, "Streaming", "No stream URL")))
        XCTAssertEqual(reading.summary, "Not streaming.")
        XCTAssertEqual(reading.activeSource, 1)
        XCTAssertEqual(reading.cameras.map(\.status), ["Streaming", "No stream URL"])
        XCTAssertEqual(reading.cameras.map(\.configured), [true, false])
    }

    func testAStreamTheCoreSwitchedOffReadsAsOffAndOneItDoesNotMentionAsOn() {
        XCTAssertEqual(videoReading(view(0, "Streaming", extra: #","streamEnabled":false"#))?.streamEnabled, false)
        XCTAssertEqual(videoReading(view(0, "Streaming"))?.streamEnabled, true)
    }

    func testTheSourceSizeIsReadOnlyWhenTheCoreReportsAUsableOne() {
        XCTAssertEqual(videoReading(view(0, "Streaming", extra: #","sourceSize":{"width":640,"height":480}"#))?.sourceSize, SourceSize(width: 640, height: 480))
        XCTAssertNil(videoReading(view(0, "Streaming", extra: #","sourceSize":{"width":0,"height":0}"#))?.sourceSize)
        XCTAssertNil(videoReading(view(0, "Streaming"))?.sourceSize)
    }

    func testThePictureIsLetterboxedInsideTheSurfaceItIsPaintedOn() {
        let wide = paintedRect(1080, 1770, SourceSize(width: 640, height: 480))
        XCTAssertEqual(wide.width, 1080, accuracy: 0.001)
        XCTAssertEqual(wide.height, 810, accuracy: 0.001)
        XCTAssertEqual(wide.left, 0, accuracy: 0.001)
        XCTAssertEqual(wide.top, 480, accuracy: 0.001)

        let tall = paintedRect(400, 200, SourceSize(width: 480, height: 640))
        XCTAssertEqual(tall.width, 150, accuracy: 0.001)
        XCTAssertEqual(tall.height, 200, accuracy: 0.001)
        XCTAssertEqual(tall.left, 125, accuracy: 0.001)
    }

    func testAnUnknownSourceSizePaintsTheWholeSurface() {
        let whole = paintedRect(300, 200, nil)
        XCTAssertEqual(whole.left, 0, accuracy: 0.001)
        XCTAssertEqual(whole.width, 300, accuracy: 0.001)
        XCTAssertEqual(whole.height, 200, accuracy: 0.001)
    }

    func testThePictureInPicturePlaysOnlyWhileAThumbnailSurfaceIsOnScreen() {
        var sent: [Bool] = []
        let surfaces = PipSurfaces { sent.append($0) }
        surfaces.created()
        surfaces.destroyed()
        XCTAssertEqual(sent, [true, false])
    }

    func testAThumbnailThatReappearsBeforeTheOldOneIsTornDownKeepsThePictureInPicturePlaying() {
        var sent: [Bool] = []
        let surfaces = PipSurfaces { sent.append($0) }
        surfaces.created()
        surfaces.created()
        surfaces.destroyed()
        XCTAssertEqual(sent, [true, true, true])
        surfaces.destroyed()
        XCTAssertEqual(sent.last, false)
    }

    func testAReplyThatIsNotTheVideoViewReadsAsNothing() {
        XCTAssertNil(videoReading(nil))
        XCTAssertNil(videoReading(JSON.parse(#"{"kind":"null"}"#)))
    }
}

final class VideoPanelVisibilityTests: XCTestCase {
    func testASourceThatIsNotConfiguredShouldNotHoldMapSpaceExplainingThat() {
        let off = videoReading(JSON.parse(#"{"class":"Video","available":false,"decoding":false,"summary":"No stream URL is set.","activeSource":0}"#))
        XCTAssertEqual(off?.available, false)
    }

    func testAConfiguredSourceThatIsNotDecodingYetStillEarnsItsPanel() {
        let waiting = videoReading(JSON.parse(#"{"class":"Video","available":true,"decoding":false,"summary":"Waiting for a stream.","activeSource":0}"#))
        XCTAssertEqual(waiting?.available, true)
    }
}
