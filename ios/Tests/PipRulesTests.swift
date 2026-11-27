import XCTest
@testable import Aircast

final class PipRulesTests: XCTestCase {
    func testTheVideoPictureInPictureNeedsVideoAndAnExpandedPipLikeFlyViewsItem2AndIsPipVisible() {
        XCTAssertEqual([videoPipShown(true, true), videoPipShown(false, true), videoPipShown(true, false)], [true, false, false])
    }

    func testThe3dViewIsOfferedOnlyWhileQgcs3dViewerIsEnabledOrAlreadyShowing() {
        XCTAssertEqual(flyViewsOffered(false, .Video), [.Video, .Map])
        XCTAssertEqual(flyViewsOffered(true, .Video), FlyView.allCases)
        XCTAssertEqual(flyViewsOffered(false, .ThreeD), [.Video, .Map])
        XCTAssertEqual(flyViewsOffered(nil, .ThreeD), FlyView.allCases)
        XCTAssertEqual(flyViewAllowed(false, .ThreeD), .Map)
        XCTAssertEqual(flyViewAllowed(nil, .ThreeD), .ThreeD)
        XCTAssertEqual(flyViewAllowed(false, .Video), .Video)
    }

    func testTheViewerSettingIsUnknownUntilTheCoreSaysSo() {
        XCTAssertNil(viewer3dEnabled(nil))
        XCTAssertNil(viewer3dEnabled(JSON.parse(#"{"kind":"object"}"#)))
        XCTAssertEqual(viewer3dEnabled(JSON.parse(#"{"enabled":true}"#)), true)
        XCTAssertEqual(viewer3dEnabled(JSON.parse(#"{"enabled":false}"#)), false)
    }

    func testThePictureIsKeptBetweenASmallThumbnailAndMostOfTheScreenAndAStoredSizeOutsideThatIsBroughtBackIn() {
        let base = CGSize(width: 320, height: 180)
        let largest = pipLargestScale(base, CGSize(width: 1200, height: 600))
        XCTAssertEqual(largest, 2.625, accuracy: 1e-9, "seven tenths of the width, unless the height runs out first")
        XCTAssertEqual(pipScaleWithin(10, largest), 2.625, accuracy: 1e-9)
        XCTAssertEqual(pipScaleWithin(0.1, largest), 0.6, "never so small it cannot be found or grabbed")
        XCTAssertEqual(pipScaleWithin(1.2, pipLargestScale(base, CGSize(width: 100, height: 50))), 0.6, "a window too small to grow it in still leaves the smallest size")
        XCTAssertEqual(scaledPip(base, 1.5), CGSize(width: 480, height: 270))
        XCTAssertEqual(storedPipScale("1.5"), 1.5)
        XCTAssertEqual(storedPipScale(nil), 1)
        XCTAssertEqual(storedPipScale("nope"), 1)
        XCTAssertEqual(storedPipScale("0"), 1)
    }

    func testTheChosenViewsAndTheMiniMapAreReadBackByName() {
        XCTAssertEqual(flyViewNamed("ThreeD"), .ThreeD)
        XCTAssertEqual(flyViewNamed(nil), .Video)
        XCTAssertEqual(flyViewSwapped(.Map), .Video)
        XCTAssertEqual(flyViewSwapped(.Video), .Map)
        XCTAssertEqual(miniMapNamed("Compass"), .Compass)
        XCTAssertEqual(miniMapNamed("bogus"), .Thumb)
    }
}
