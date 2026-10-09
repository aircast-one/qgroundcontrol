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

    func testASilentLinkSaysLoadingStoppedInsteadOfPromisingProgress() {
        let lost = loadingWords("guta-test", lost: true)
        XCTAssertEqual(lost.title, "Signal lost")
        XCTAssertTrue(lost.detail.hasPrefix("Loading stopped."))
        XCTAssertEqual(lost.dismiss, "Hide")
        XCTAssertEqual(loadingWords("guta-test", lost: false).title, "Connecting to guta-test")
    }

    func testTheEmptyFlyViewSaysWhatToDoNext() {
        XCTAssertEqual(LOOKING_TITLE, "Looking for your aircraft")
        XCTAssertTrue(LOOKING_HINT.hasPrefix("Turn on the aircraft"))
        XCTAssertFalse(LOOKING_HINT.contains("USB"), "iOS has no USB serial links, so the hint must not promise one")
    }
}
