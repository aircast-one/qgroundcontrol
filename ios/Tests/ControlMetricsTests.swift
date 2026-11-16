import SwiftUI
import XCTest
@testable import Aircast

private let SIZES: [KeyPath<ControlMetrics, CGFloat>] = [
    \.rowLarge, \.rowTall, \.row, \.rowCompact, \.rowDense, \.floatingButton, \.primaryButton,
    \.iconButton, \.iconButtonSmall, \.avatar, \.stepper, \.minTarget,
]

final class ControlMetricsTests: XCTestCase {
    func testTouchMetricsKeepTheSizesTheIosAppHasAlwaysUsed() {
        XCTAssertEqual(SIZES.map { ControlMetrics.touch[keyPath: $0] }, [72, 64, 56, 48, 40, 56, 52, 48, 40, 40, 48, 48])
        XCTAssertEqual(ControlMetrics.touch.minTarget, MINIMUM_TOUCH_TARGET)
    }

    func testTheAppReadsTheTouchMetrics() {
        XCTAssertEqual(Metrics.current, .touch)
        XCTAssertEqual(Metrics.row, 56)
    }
}
