import XCTest
@testable import Aircast

final class OverlayLayoutTests: XCTestCase {
    func testHiddenWidgetsAreReadFromTheStoredRigKeysOnly() {
        let stored: [String: Any] = ["OverlayRigHidden-orbit": true, "OverlayRigHidden-traffic": false, "other": true]
        XCTAssertEqual(hiddenKeys(stored), ["orbit"])
    }

    func testHidingAndShowingTogglesOneKey() {
        XCTAssertEqual(withHidden(["a"], "b", true), ["a", "b"])
        XCTAssertEqual(withHidden(["a"], "a", false), [])
    }

    func testResetLayoutArmsOnTheFirstTapAndResetsOnTheSecondLikeResetPill() {
        XCTAssertEqual(resetTap(false), ResetTap(reset: false, armed: true))
        XCTAssertEqual(resetTap(true), ResetTap(reset: true, armed: false))
        XCTAssertEqual(resetPillText(false), "Reset layout")
        XCTAssertEqual(resetPillText(true), "Tap again to reset")
    }

    func testADraggedWidgetStopsAtTheScreenEdges() {
        XCTAssertEqual(clampedDrag(100, 40, 200, 140, 1000, 2000, 30, -80), CGSize(width: 30, height: -40))
        XCTAssertEqual(clampedDrag(100, 40, 200, 140, 1000, 2000, -500, 5000), CGSize(width: -100, height: 1860))
        XCTAssertEqual(clampedDrag(100, 40, 200, 140, 1000, 2000, 900, 0), CGSize(width: 800, height: 0))
    }

    func testStoredOffsetsAreReadBackPerWidgetAndMalformedOnesAreDropped() {
        let stored: [String: Any] = ["OverlayRigOffset-instrumentPanel": "12.5,-30.0", "OverlayRigOffset-orbit": "bad", "OverlayRigHidden-traffic": true]
        XCTAssertEqual(storedOffsets(stored), ["instrumentPanel": CGSize(width: 12.5, height: -30)])
    }

    func testTheStoredLayoutYieldsHiddenWidgetsOffsetsAndTheIndicatorOrder() {
        let stored: [String: Any] = ["OverlayRigOffset-instrumentPanel": "10.0,20.0", "OverlayRigHidden-orbit": true, "FlyViewIndicatorOrder": "gps,battery"]
        XCTAssertEqual(storedOffsets(stored), ["instrumentPanel": CGSize(width: 10, height: 20)])
        XCTAssertEqual(hiddenKeys(stored), ["orbit"])
        XCTAssertEqual(storedIndicatorOrder(stored), ["gps", "battery"])
    }

    func testPortraitAndLandscapeKeepSeparateOffsets() {
        XCTAssertEqual(orientedKey("instrumentPanel", false), "instrumentPanel")
        XCTAssertEqual(orientedKey("instrumentPanel", true), "instrumentPanel@landscape")
    }

    func testAWidgetItsOwnOffsetPushedOffScreenIsPulledBackAScrolledOrOversizedOneIsLeftAlone() {
        let root = CGSize(width: 1000, height: 500)
        func rect(_ left: CGFloat, _ top: CGFloat, _ right: CGFloat, _ bottom: CGFloat) -> CGRect { CGRect(x: left, y: top, width: right - left, height: bottom - top) }
        XCTAssertEqual(onScreenCorrection(rect(900, 400, 1300, 600), root, CGSize(width: 400, height: 300)), CGSize(width: -300, height: -100))
        XCTAssertEqual(onScreenCorrection(rect(900, 0, 1300, 100), root, CGSize(width: 100, height: 0)), CGSize(width: -100, height: 0))
        XCTAssertEqual(onScreenCorrection(rect(-40, 0, 60, 100), root, CGSize(width: -80, height: 0)), CGSize(width: 40, height: 0))
        XCTAssertEqual(onScreenCorrection(rect(1100, 10, 1300, 200), root, .zero), .zero)
        XCTAssertEqual(onScreenCorrection(rect(10, 10, 200, 200), root, CGSize(width: 50, height: 50)), .zero)
        XCTAssertEqual(onScreenCorrection(rect(20, 10, 1600, 200), root, CGSize(width: 30, height: 0)), .zero)
    }

    func testSavedOrderComesFirstAndNewIndicatorsFollowInTheirOwnOrder() {
        XCTAssertEqual(orderedKeys(["battery", "gps", "rc"], ["gps", "gone", "battery"]), ["gps", "battery", "rc"])
    }

    func testAnIndicatorMovesOneSlotAndNotPastEitherEnd() {
        XCTAssertEqual(movedKey(["battery", "gps", "rc"], "battery", 1), ["gps", "battery", "rc"])
        XCTAssertEqual(movedKey(["battery", "gps", "rc"], "rc", -1), ["battery", "rc", "gps"])
        XCTAssertNil(movedKey(["battery", "gps"], "battery", -1))
        XCTAssertNil(movedKey(["battery", "gps"], "nope", 1))
    }
}
