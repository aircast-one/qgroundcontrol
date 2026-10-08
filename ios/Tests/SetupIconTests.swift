import XCTest
@testable import Aircast

final class SetupIconTests: XCTestCase {
    func testAKnownPageKeepsItsIcon() {
        XCTAssertEqual(setupIcon("radio", className: "APMRadioComponent"), .gamepad)
    }

    func testAnUnknownPageFallsBackToItsUntranslatedClass() {
        XCTAssertEqual(setupIcon(nil, className: "APMFailsafesComponent"), .warning)
        XCTAssertEqual(setupIcon(nil, className: "APMAdvancedTuningCopterComponent"), .tune)
        XCTAssertEqual(setupIcon(nil, className: "SomethingNewComponent"), .build)
    }

    func testTheNoteFollowsTheMostSpecificClassToken() {
        XCTAssertEqual(setupNote("APMAdvancedTuningCopterComponent"), "Every rate and filter, per axis")
        XCTAssertEqual(setupNote("APMTuningComponent"), "How it responds to the sticks")
        XCTAssertEqual(setupNote("APMFlightSafetyComponent"), "Return altitude, landing speed and geofence")
        XCTAssertEqual(setupNote("SomethingNewComponent"), "")
    }
}
