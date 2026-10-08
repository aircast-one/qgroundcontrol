import XCTest
@testable import Aircast

final class MissionSummaryTests: XCTestCase {
    private func row(_ row: (String, String)) -> String { #"{"label":"\#(row.0)","value":"\#(row.1)"}"# }

    private func view(_ rows: (String, String)...) -> JSON {
        JSON.parse(#"{"rows":["# + rows.map(row).joined(separator: ",") + "]}")
    }

    func testTheSummaryReadsTheCoresWordsRatherThanFormattingItsOwn() {
        XCTAssertEqual("Distance 4.82 km · Time 19:37", missionSummaryText(view(("Distance", "4.82 km"), ("Time", "19:37"))))
    }

    func testImperialUnitsComeThroughUntouchedBecauseTheCoreAlreadyConvertedThem() {
        XCTAssertEqual("Distance 2.99 miles · Time 19:37", missionSummaryText(view(("Distance", "2.99 miles"), ("Time", "19:37"))))
    }

    func testARowTheCoreLeftOutIsLeftOutHere() {
        XCTAssertEqual("Distance 4.82 km", missionSummaryText(view(("Distance", "4.82 km"))))
        XCTAssertEqual("Time 19:37", missionSummaryText(view(("Time", "19:37"))))
    }

    func testRowsTheSummaryDoesNotShowAreIgnored() {
        XCTAssertEqual("Distance 4.82 km", missionSummaryText(view(("Cruise", "3 km"), ("Distance", "4.82 km"), ("Hover", "1 km"))))
    }

    func testNoPlanIsAnEmptySummaryNotAStraySeparator() {
        XCTAssertEqual("", missionSummaryText(JSON.parse(#"{"rows":[]}"#)))
        XCTAssertEqual("", missionSummaryText(nil))
    }

    func testTheFurthestPointFromLaunchReadsAsQgcsMaxTelemEachTotalLabelledLikePlanToolBarIndicators() {
        XCTAssertEqual(
            "Distance 4.82 km · Time 19:37 · Max telem 1.2 km",
            missionSummaryText(view(("Distance", "4.82 km"), ("Time", "19:37"), ("Furthest from launch", "1.2 km")))
        )
    }
}
