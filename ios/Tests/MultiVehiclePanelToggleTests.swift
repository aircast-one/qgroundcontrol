import XCTest
@testable import Aircast

final class MultiVehiclePanelToggleTests: XCTestCase {
    func testTheToggleFollowsTheFactsVisibleFlagAsMultiVehicleSelectorDoes() {
        XCTAssertEqual(panelToggleShown(JSON.parse(#"{"visible":false}"#)), false)
        XCTAssertEqual(panelToggleShown(JSON.parse(#"{"visible":true}"#)), true)
        XCTAssertEqual(panelToggleShown(nil), true)
    }
}
