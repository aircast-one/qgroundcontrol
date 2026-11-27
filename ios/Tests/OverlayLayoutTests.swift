import XCTest
@testable import Aircast

final class OverlayLayoutTests: XCTestCase {
    func testHiddenWidgetsAreReadFromTheStoredRigKeysOnly() {
        let stored: [String: Any] = ["OverlayRigHidden-orbit": true, "OverlayRigHidden-traffic": false, "other": true]
        XCTAssertEqual(hiddenKeys(stored), ["minimap", "orbit"])
    }

    func testTheMiniMapStartsHiddenAndStaysShownOnceThePilotShowsIt() {
        XCTAssertEqual(hiddenKeys([:]), ["minimap"])
        XCTAssertEqual(hiddenKeys(["OverlayRigHidden-minimap": false]), [])
    }

    func testASmallWidgetJigglesTheFullAngleAndAWideOneTurnsLessSoNoCornerSwingsMoreThanAFewPoints() {
        XCTAssertEqual(jiggleDegrees(48, 48), 1.2)
        XCTAssertEqual(550 * jiggleDegrees(1100, 12) * .pi / 180, 4, accuracy: 0.1)
    }

    func testANamedBadgeSaysWhatItShowsOrHides() {
        XCTAssertEqual(badgeLabel(true, "Mini map"), "Show mini map")
        XCTAssertEqual(badgeLabel(false, nil), "Hide")
    }

    func testTheBatteryBarButtonSaysWhetherTheBarIsShown() {
        XCTAssertEqual(batteryBarPillText(false), "Battery bar: Shown")
        XCTAssertEqual(batteryBarPillText(true), "Battery bar: Hidden")
    }

    func testResetHidesTheMiniMapAgain() {
        let layout = OverlayLayoutState(overlayLayoutStore())
        layout.reset()
        layout.setHidden(MINIMAP_KEY, false)
        layout.setHidden(BATTERY_BAR_KEY, true)
        XCTAssertEqual(layout.hidden, [BATTERY_BAR_KEY])
        XCTAssertEqual(OverlayLayoutState(overlayLayoutStore()).hidden, [BATTERY_BAR_KEY])
        layout.reset()
        XCTAssertEqual(layout.hidden, [MINIMAP_KEY])
    }

    func testAnItemStepsBiggerOrSmallerThanNormalAndStopsAtEachEndItsSizeIsStoredAndAResetDropsIt() {
        let layout = OverlayLayoutState(overlayLayoutStore())
        layout.reset()
        XCTAssertEqual([0.7, 1, 1.5, 2].map(largerItemSize), [0.85, 1.25, 2, nil])
        XCTAssertEqual([0.7, 0.85, 1, 2].map(smallerItemSize), [nil, 0.7, 0.85, 1.5], "below normal is where a crowded screen needs it")
        layout.resize(HEADING_TAPE_KEY, 0.85)
        XCTAssertEqual(layout.sizes, [HEADING_TAPE_KEY: 0.85])
        XCTAssertEqual(OverlayLayoutState(overlayLayoutStore()).sizes, [HEADING_TAPE_KEY: 0.85])
        XCTAssertEqual(storedSizes(["OverlayRigSize-headingTape": "0.7", "OverlayRigSize-orbit": "9", "OverlayRigOffset-orbit": "1,1"]), ["headingTape": 0.7])
        layout.reset()
        XCTAssertEqual(layout.sizes, [:])
        XCTAssertEqual(OverlayLayoutState(overlayLayoutStore()).sizes, [:])
    }

    func testAHeldControlKeepsHoldToEditAwayUntilEveryHoldEnds() {
        let layout = OverlayLayoutState(overlayLayoutStore())
        layout.holdGesture(true)
        layout.holdGesture(true)
        layout.holdGesture(false)
        XCTAssertTrue(layout.gestureHeld)
        layout.holdGesture(false)
        layout.holdGesture(false)
        XCTAssertFalse(layout.gestureHeld)
        layout.holdGesture(true)
        XCTAssertTrue(layout.gestureHeld)
    }

    func testHidingAndShowingTogglesOneKey() {
        XCTAssertEqual(withHidden(["a"], "b", true), ["a", "b"])
        XCTAssertEqual(withHidden(["a"], "a", false), [])
    }

    func testResetLayoutArmsOnTheFirstTapAndResetsOnTheSecondLikeResetPill() {
        XCTAssertEqual(resetTap(false), ResetTap(reset: false, armed: true))
        XCTAssertEqual(resetTap(true), ResetTap(reset: true, armed: false))
        XCTAssertEqual(resetPillText(false), "Reset positions and hidden items")
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
        XCTAssertEqual(hiddenKeys(stored), ["minimap", "orbit"])
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

    func testAnIndicatorStepsPastEmptyOnesAndTheyKeepTheirSlots() {
        let keys = ["battery", "rc", "esc", "gps", "joystick"]
        let centres: [String: CGFloat] = ["battery": 20, "gps": 80]
        XCTAssertEqual(movedAmongShown(keys, centres, "gps", -1), ["gps", "rc", "esc", "battery", "joystick"])
        XCTAssertEqual(movedAmongShown(keys, centres, "battery", 1), ["gps", "rc", "esc", "battery", "joystick"])
        XCTAssertNil(movedAmongShown(keys, centres, "gps", 1))
    }

    func testADroppedIndicatorLandsWhereItWasLetGo() {
        let keys = ["battery", "rc", "gps", "links"]
        let centres: [String: CGFloat] = ["battery": 20, "gps": 80, "links": 140]
        XCTAssertEqual(droppedOrder(keys, centres, "battery", 90), ["gps", "rc", "battery", "links"])
        XCTAssertEqual(droppedOrder(keys, centres, "links", -200), ["links", "rc", "battery", "gps"])
        XCTAssertEqual(droppedOrder(keys, centres, "gps", 10), keys)
    }
}
