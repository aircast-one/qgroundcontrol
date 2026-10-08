import XCTest
@testable import Aircast

final class AdvancedUiTests: XCTestCase {
    func testAdvancedModeIsOnUntilTheCoreSaysOtherwiseLikeQGCCorePlugin() {
        XCTAssertTrue(advancedUiShown(nil))
        XCTAssertFalse(advancedUiShown(JSON.parse(#"{"shown":false}"#)))
    }
}
