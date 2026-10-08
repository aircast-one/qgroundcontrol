import XCTest
@testable import Aircast

final class VideoFitTests: XCTestCase {
    private let wide = 16.0 / 9.0

    private func assertSize(_ actual: (CGFloat, CGFloat), _ width: CGFloat, _ height: CGFloat, _ message: String = "") {
        XCTAssertEqual(actual.0, width, accuracy: 0.01, message)
        XCTAssertEqual(actual.1, height, accuracy: 0.01, message)
    }

    func testEachFitModeSizesThePictureLikeFlightDisplayViewVideo() {
        assertSize(videoContentSize(1000, 1000, wide, VIDEO_FIT_WIDTH), 1000, 562.5)
        assertSize(videoContentSize(1000, 1000, wide, VIDEO_FIT_HEIGHT), 1777.7778, 1000)
        assertSize(videoContentSize(1000, 1000, wide, VIDEO_FILL), 1777.7778, 1000, "a square box is filled by cropping the sides")
        assertSize(videoContentSize(1000, 1000, wide, VIDEO_NO_CROP), 1000, 562.5, "no crop letterboxes")
        assertSize(videoContentSize(1000, 400, 0, VIDEO_NO_CROP), 1000, 400, "no aspect known fills the box")
    }

    func testTheStreamsOwnSizeWinsOverTheAspectRatioSetting() {
        XCTAssertEqual(videoAspect(SourceSize(width: 640, height: 480), wide), 4.0 / 3.0, accuracy: 1e-9)
        XCTAssertEqual(videoAspect(nil, wide), wide, accuracy: 1e-9)
        XCTAssertEqual(videoAspect(SourceSize(width: 0, height: 0), nil), 0, accuracy: 1e-9)
    }
}
