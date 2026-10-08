import XCTest
@testable import Aircast

final class AutotuneSectionTests: XCTestCase {
    func testAutotuneStateAndTheTuningFlightModesReadFromTheirViews() {
        XCTAssertEqual(
            autotuneState(JSON.parse(#"{"available":true,"canStart":false,"status":"Take off first - auto-tuning runs in flight","progress":0,"warning":"WARNING!"}"#)),
            AutotuneState(canStart: false, status: "Take off first - auto-tuning runs in flight", progress: 0, warning: "WARNING!")
        )
        XCTAssertNil(autotuneState(JSON.parse(#"{"available":false}"#)))
        XCTAssertEqual(tuningModes(JSON.parse(#"{"stabilizedFlightMode":"Stabilized","pauseFlightMode":"Hold"}"#)), TuningModes(stabilized: "Stabilized", pause: "Hold"))
    }
}
