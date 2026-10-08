import XCTest
@testable import Aircast

final class SectionMatchesTests: XCTestCase {
    private let gcs = ParameterRows(title: "Ground Station Failsafe", facts: [], note: "", keywords: ["heartbeat", "fs_gcs_timeout"])

    func testMatchesTitleOrKeywordIgnoringCase() {
        XCTAssertTrue(sectionMatches(gcs, "  station "))
        XCTAssertTrue(sectionMatches(gcs, "HEARTBEAT"))
        XCTAssertTrue(sectionMatches(gcs, ""))
        XCTAssertFalse(sectionMatches(gcs, "battery"))
    }
}
