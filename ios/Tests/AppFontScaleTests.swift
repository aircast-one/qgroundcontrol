import XCTest
@testable import Aircast

final class AppFontScaleTests: XCTestCase {
    func testFontSizeScalesFromThePlatformDefaultAndUnsetKeepsIt() {
        XCTAssertEqual(appFontScale(14), 1, accuracy: 0.001)
        XCTAssertEqual(appFontScale(21), 1.5, accuracy: 0.001)
        XCTAssertEqual(appFontScale(0), 1, accuracy: 0.001)
        XCTAssertEqual(appFontScale(60), 1, accuracy: 0.001)
        XCTAssertEqual(appFontScale(nil), 1, accuracy: 0.001)
    }
}
