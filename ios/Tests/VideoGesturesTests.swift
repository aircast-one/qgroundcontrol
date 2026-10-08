import XCTest
@testable import Aircast

final class VideoGesturesTests: XCTestCase {
    func testASwipePastTheThresholdReadsAsItsMainDirectionAndAShortOneIsIgnored() {
        XCTAssertEqual(videoSwipe(CGSize(width: 5, height: -80), 40), .Up)
        XCTAssertEqual(videoSwipe(CGSize(width: -10, height: 60), 40), .Down)
        XCTAssertNil(videoSwipe(CGSize(width: 0, height: -30), 40), "too short")
        XCTAssertEqual([VideoSwipe.Left, .Right, .Up].map(cameraStep), [1, -1, nil])
        XCTAssertEqual(videoSwipe(CGSize(width: 120, height: -60), 40), .Right, "sideways switches the camera rather than hiding")
        XCTAssertEqual(videoSwipe(CGSize(width: -90, height: 10), 40), .Left)
    }

    func testADraggedPictureInPictureSettlesOnTheSideItsCentreWasDroppedOn() {
        XCTAssertTrue(pipOnStart(300, 1080))
        XCTAssertFalse(pipOnStart(700, 1080))
    }

    func testOnlyASidewaysMoveCountsAsSideways() {
        XCTAssertTrue(sideways(CGSize(width: 50, height: 10)))
        XCTAssertFalse(sideways(CGSize(width: 10, height: 50)))
    }
}
