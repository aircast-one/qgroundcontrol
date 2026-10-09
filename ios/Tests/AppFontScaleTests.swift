import SwiftUI
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

    func testTheScaleMovesTheSystemTextSizeToTheNearestStep() {
        XCTAssertEqual(scaledTypeSize(.large, 1), .large)
        XCTAssertEqual(scaledTypeSize(.large, 2), .accessibility2)
        XCTAssertEqual(scaledTypeSize(.medium, 0.9), .xSmall)
        XCTAssertEqual(scaledTypeSize(.xSmall, 0.1), .xSmall)
        XCTAssertEqual(scaledTypeSize(.accessibility5, 4), .accessibility5)
    }
}
