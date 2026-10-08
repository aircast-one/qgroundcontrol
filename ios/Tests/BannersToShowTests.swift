import XCTest
@testable import Aircast

final class BannersToShowTests: XCTestCase {
    private let now: Int64 = 1_000_000
    private let missing = "Aircast · Parameters are missing"
    private let ekf = "Aircast · EKF variance"

    func testAMessageArrivingManyTimesInOneBatchIsShownOnce() {
        XCTAssertEqual(quietBanners([missing, missing], [:], now), [missing])
    }

    func testAMessageThatRepeatsFasterThanTheQuietWindowIsHeldBack() {
        XCTAssertEqual(quietBanners([missing], [missing: now - 1_000], now), [])
    }

    func testAConditionStillRecurringAfterTheQuietWindowIsAnnouncedAgain() {
        XCTAssertEqual(quietBanners([ekf], [ekf: now - REPEAT_QUIET_MS - 1], now), [ekf])
    }

    func testOneMessageBeingHeldBackDoesNotHoldBackADifferentOne() {
        XCTAssertEqual(quietBanners([missing, ekf], [missing: now - 1_000], now), [ekf])
    }
}
