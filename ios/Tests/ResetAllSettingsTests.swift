import XCTest
@testable import Aircast

final class ResetAllSettingsTests: XCTestCase {
    func testAPendingResetReadsFromTheSettingValue() {
        XCTAssertTrue(resetPending(.bool(true)))
        XCTAssertFalse(resetPending(.bool(false)))
        XCTAssertFalse(resetPending(nil))
    }
}
