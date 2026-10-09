import XCTest
@testable import Aircast

final class GuidedCompactTests: XCTestCase {
    func testALandscapePhoneFoldsThePanelSoTheHoldButtonNeverPushesTheValueOutOfSight() {
        XCTAssertEqual([true, false, false], [guidedCompact(false, 344), guidedCompact(false, 600), guidedCompact(true, 344)])
    }

    func testTheFoldedPanelCarriesTheRangeBesideTheLabelInsteadOfALineOfItsOwn() {
        XCTAssertEqual("Height above launch \u{00b7} 3.0 to 100.0 m", guidedLabel("Height above launch", "3.0 to 100.0 m", compact: true))
        XCTAssertEqual("Height above launch", guidedLabel("Height above launch", "3.0 to 100.0 m", compact: false))
        XCTAssertEqual("Height above launch", guidedLabel("Height above launch", nil, compact: true))
    }
}
