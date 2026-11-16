import XCTest
@testable import Aircast

final class ShellPortraitTests: XCTestCase {
    func testOnlyAPhoneShapedIosWindowTakesThePortraitLayout() {
        let tall = CGSize(width: 800, height: 1200)
        let wide = CGSize(width: 1280, height: 800)
        XCTAssertFalse(shellIsPortrait(wide))
        XCTAssertTrue(shellIsPortrait(tall))
    }
}

