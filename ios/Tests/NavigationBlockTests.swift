import XCTest
@testable import Aircast

final class NavigationBlockTests: XCTestCase {
    func testASetReasonRefusesLeavingWithQGCsWordsLikeMainWindowAllowViewSwitch() {
        XCTAssertEqual(navigationRefusal(LOG_DOWNLOAD_BLOCK, true), LOG_DOWNLOAD_BLOCK)
        XCTAssertNil(navigationRefusal(nil, true))
        XCTAssertNil(navigationRefusal(CALIBRATION_BLOCK, false))
        XCTAssertEqual(CALIBRATION_BLOCK, "Complete or cancel the current calibration first")
    }
}
