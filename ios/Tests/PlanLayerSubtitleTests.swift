import XCTest
@testable import Aircast

final class PlanLayerSubtitleTests: XCTestCase {
    func testLayerSubtitlesReadLikePlanTreeViewGroupHeaders() {
        XCTAssertEqual("1 items", layerSubtitle(.Mission, 1, 0))
        XCTAssertEqual("0 items", layerSubtitle(.Mission, 0, 0))
        XCTAssertEqual("3 points", layerSubtitle(.Rally, 0, 3))
        XCTAssertEqual("", layerSubtitle(.Fence, 4, 2))
    }
}
