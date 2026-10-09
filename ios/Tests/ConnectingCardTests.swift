import XCTest
@testable import Aircast

final class ConnectingCardTests: XCTestCase {
    func testNamesTheVehicleWhenKnown() {
        XCTAssertEqual(connectingTitle("guta-test"), "Connecting to guta-test")
    }

    func testFallsBackWithoutAName() {
        XCTAssertEqual(connectingTitle(nil), "Connecting")
        XCTAssertEqual(connectingTitle(""), "Connecting")
    }

    func testTheEmptyFlyViewSaysWhatToDoNext() {
        XCTAssertEqual(LOOKING_TITLE, "Looking for your aircraft")
        XCTAssertTrue(LOOKING_HINT.hasPrefix("Turn on the aircraft"))
        XCTAssertFalse(LOOKING_HINT.contains("USB"), "iOS has no USB serial links, so the hint must not promise one")
    }
}
