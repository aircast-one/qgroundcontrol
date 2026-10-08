import XCTest
@testable import Aircast

final class FlyPortraitTests: XCTestCase {
    private func reading(_ available: Bool, _ streamEnabled: Bool, _ sourceChosen: Bool) -> VideoReading {
        VideoReading(available: available, decoding: false, sourceSize: nil, summary: "", activeSource: 0, cameras: [], streamEnabled: streamEnabled, sourceChosen: sourceChosen)
    }

    func testTheCameraPaneStaysInPortraitWhileVideoIsSwitchedOffSoItCanBeTurnedBackOn() {
        XCTAssertTrue(portraitShowsCamera(reading(true, true, true)))
        XCTAssertTrue(portraitShowsCamera(reading(false, false, true)))
        XCTAssertFalse(portraitShowsCamera(reading(false, true, false)))
        XCTAssertFalse(portraitShowsCamera(reading(false, false, false)))
        XCTAssertFalse(portraitShowsCamera(nil))
    }

    func testTheMapButtonsDropBelowTheVideoThumbnailOnlyWhenTheThumbnailIsDrawn() {
        XCTAssertTrue(portraitVideoThumbnail(false, reading(true, true, true)))
        XCTAssertFalse(portraitVideoThumbnail(false, reading(false, false, true)), "video off draws no thumbnail in the map view")
        XCTAssertFalse(portraitVideoThumbnail(true, reading(true, true, true)), "the split view has the picture above, not a thumbnail")
        XCTAssertFalse(portraitVideoThumbnail(false, nil))
    }

    func testASwipePastTheThresholdReadsAsItsMainDirectionAndAShortOneIsIgnored() {
        XCTAssertEqual(videoSwipe(CGSize(width: 5, height: -80), 40), .Up)
        XCTAssertEqual(videoSwipe(CGSize(width: -10, height: 60), 40), .Down)
        XCTAssertNil(videoSwipe(CGSize(width: 0, height: -30), 40), "too short")
        XCTAssertEqual([VideoSwipe.Left, .Right, .Up].map { cameraStep($0) }, [1, -1, nil])
        XCTAssertEqual(videoSwipe(CGSize(width: 120, height: -60), 40), .Right, "sideways switches the camera rather than hiding")
        XCTAssertEqual(videoSwipe(CGSize(width: -90, height: 10), 40), .Left)
    }

    func testADraggedPictureInPictureSettlesOnTheSideItsCentreWasDroppedOn() {
        XCTAssertTrue(pipOnStart(300, 1080))
        XCTAssertFalse(pipOnStart(700, 1080))
        let geometry = PipGeometry(width: 1080, pip: CGSize(width: 440, height: 248), inset: 33, pipTop: 300, split: CGRect(x: 0, y: 200, width: 1080, height: 608), full: CGRect(x: 0, y: 0, width: 1080, height: 2200))
        XCTAssertEqual(geometry.anchor(false), CGPoint(x: 607, y: 300))
        XCTAssertEqual(geometry.anchor(true), CGPoint(x: 33, y: 300))
        let finger = CGPoint(x: 540, y: 500)
        let pulled = geometry.pip(false, geometry.dragToCentre(false, finger))
        XCTAssertEqual(CGPoint(x: pulled.midX, y: pulled.midY), finger, "pulled out of the split view, the picture is centred under the finger")
    }

    func testTheControlsBesideTheVideoMakeRoomForThePictureOrForItsTab() {
        XCTAssertEqual(pipRoom(false, false), 0)
        XCTAssertLessThan(pipRoom(true, true), pipRoom(true, false))
    }
}
