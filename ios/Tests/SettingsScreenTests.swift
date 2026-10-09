import XCTest
@testable import Aircast

final class SettingsScreenTests: XCTestCase {
    func testOptionsSplitIntoRunsWhereTheirGroupChanges() {
        XCTAssertEqual(optionRuns(5, ["", "", "A", "A", "B"]), [
            OptionRun(id: 0, group: "", indices: 0..<2),
            OptionRun(id: 2, group: "A", indices: 2..<4),
            OptionRun(id: 4, group: "B", indices: 4..<5),
        ])
    }

    func testOptionsPastTheGroupListFallIntoAnUngroupedRun() {
        XCTAssertEqual(optionRuns(4, ["A"]), [
            OptionRun(id: 0, group: "A", indices: 0..<1),
            OptionRun(id: 1, group: "", indices: 1..<4),
        ])
    }

    func testNoOptionsMakeNoRuns() {
        XCTAssertEqual(optionRuns(0, ["A"]), [])
    }
}
