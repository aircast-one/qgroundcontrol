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

    func testDesktopMetricsAreDenserThanTouch() {
        SIZES.forEach { XCTAssertLessThan(ControlMetrics.desktop[keyPath: $0], ControlMetrics.touch[keyPath: $0]) }
    }

    func testDesktopMetricsStayLargeEnoughToPointAt() {
        SIZES.forEach { XCTAssertGreaterThanOrEqual(ControlMetrics.desktop[keyPath: $0], 28) }
    }

    func testEachPlatformReadsItsOwnMetrics() {
        #if os(macOS)
        XCTAssertEqual(Metrics.current, .desktop)
        XCTAssertEqual(Metrics.row, 36)
        #else
        XCTAssertEqual(Metrics.current, .touch)
        XCTAssertEqual(Metrics.row, 56)
        #endif
    }

    func testTheCursorIsPushedOnceWhileOverAnEnabledControl() {
        XCTAssertEqual(hoverCursor(hovering: true, enabled: true, pushed: false), .push)
        XCTAssertEqual(hoverCursor(hovering: true, enabled: true, pushed: true), .keep)
        XCTAssertEqual(hoverCursor(hovering: false, enabled: true, pushed: true), .pop)
        XCTAssertEqual(hoverCursor(hovering: false, enabled: true, pushed: false), .keep)
    }

    func testDisablingAHoveredControlGivesTheCursorBack() {
        XCTAssertEqual(hoverCursor(hovering: true, enabled: false, pushed: true), .pop)
        XCTAssertEqual(hoverCursor(hovering: true, enabled: false, pushed: false), .keep)
    }
}
