import SwiftUI
import XCTest
@testable import Aircast

final class SilenceTests: XCTestCase {
    func testAHeardVehicleDrawsOnceAndALostOneCountsEverySecond() {
        let start = Date(timeIntervalSince1970: 1_000)
        XCTAssertEqual([start], Array(SilenceSchedule(ticking: false).entries(from: start, mode: .normal)))
        XCTAssertEqual(
            [start, start.addingTimeInterval(1), start.addingTimeInterval(2)],
            Array(SilenceSchedule(ticking: true).entries(from: start, mode: .normal).prefix(3))
        )
    }
}
