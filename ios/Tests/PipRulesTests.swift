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

    func testTheChosenViewsAndTheMiniMapAreReadBackByName() {
        XCTAssertEqual(flyViewNamed("ThreeD"), .ThreeD)
        XCTAssertEqual(flyViewNamed(nil), .Video)
        XCTAssertEqual(flyViewSwapped(.Map), .Video)
        XCTAssertEqual(flyViewSwapped(.Video), .Map)
        XCTAssertEqual(miniMapNamed("Compass"), .Compass)
        XCTAssertEqual(miniMapNamed("bogus"), .Thumb)
    }
}
