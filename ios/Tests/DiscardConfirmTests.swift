import XCTest
@testable import Aircast

final class DiscardConfirmTests: XCTestCase {
    func testDirtyAndSyncingComeOffThePlanViewTheTabAlreadyReads() {
        let idle = JSON.parse(#"{"kind":"object","class":"PlanStatus","dirty":false,"sync":{"state":"ready"}}"#)
        let busy = JSON.parse(#"{"kind":"object","class":"PlanStatus","dirty":true,"sync":{"state":"busy"}}"#)
        XCTAssertFalse(planIsDirty(idle))
        XCTAssertTrue(planIsDirty(busy))
        XCTAssertFalse(planIsSyncing(idle))
        XCTAssertTrue(planIsSyncing(busy), "plan.rs serves busy for a sync in flight; the head compared against a syncing it never sends")
        XCTAssertFalse(planIsSyncing(JSON.parse(#"{"kind":"object","sync":{"state":"offline"}}"#)))
        XCTAssertTrue(planContainsItems(JSON.parse(#"{"kind":"object","containsItems":true}"#)))
        XCTAssertFalse(planContainsItems(nil))
    }

    func testNoPlanViewIsNotACleanPlanAndNotAFinishedSync() {
        XCTAssertFalse(planIsDirty(nil))
        XCTAssertFalse(planIsSyncing(nil))
        XCTAssertFalse(planIsSyncing(JSON.parse(#"{"kind":"object","class":"PlanStatus"}"#)))
    }

    func testSyncProgressComesOffThePlanViewClampedAndReadsZeroWithoutOne() {
        XCTAssertEqual(0.4, planSyncProgress(JSON.parse(#"{"sync":{"state":"busy","progress":0.4}}"#)), accuracy: 1e-6)
        XCTAssertEqual(1, planSyncProgress(JSON.parse(#"{"sync":{"progress":3}}"#)))
        XCTAssertEqual(0, planSyncProgress(nil))
    }
}
