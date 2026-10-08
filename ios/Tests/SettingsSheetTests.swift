import XCTest
@testable import Aircast

final class SettingsSheetTests: XCTestCase {
    func testAPageOpensOnTheTabThatHoldsIt() {
        XCTAssertEqual(openingGroup("Connections"), .Transmission)
        XCTAssertEqual(openingGroup("Video"), .Camera)
        XCTAssertEqual(openingGroup(nil), .Safety)
    }

    func testTheTabsGiveWayToATitleOnceAPageOrSetupIsOpen() {
        XCTAssertFalse(sheetDrilled(nil, false))
        XCTAssertTrue(sheetDrilled("Connections", false))
        XCTAssertTrue(sheetDrilled(nil, true))
    }
}
