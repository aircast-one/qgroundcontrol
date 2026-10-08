import XCTest
@testable import Aircast

final class LoadConfirmTests: XCTestCase {
    func testACleanPlanLoadsWithoutAsking() {
        XCTAssertEqual(loadStep(false, false), .Load)
    }

    func testUnsentChangesAreNotDiscardedOnTheFirstTap() {
        XCTAssertEqual(loadStep(true, false), .Confirm)
    }

    func testTheSecondTapGoesThrough() {
        XCTAssertEqual(loadStep(true, true), .Load)
    }

    func testAnArmedConfirmOnACleanPlanStillJustLoads() {
        XCTAssertEqual(loadStep(false, true), .Load)
    }

    func testADirtyPlanAsksEvenWithNoMissionItemsAsDownloadClickedChecksOnlyDirty() {
        XCTAssertEqual(loadStep(true, false), .Confirm)
    }

    func testTheReplaceWarningCountsThePlanThatWouldBeLost() {
        XCTAssertEqual(replaceWarning(6), "Your unsaved plan here (6 items) will be replaced.")
        XCTAssertEqual(replaceWarning(1), "Your unsaved plan here (1 item) will be replaced.")
    }
}
