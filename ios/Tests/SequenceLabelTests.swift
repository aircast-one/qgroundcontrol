import XCTest
@testable import Aircast

final class SequenceLabelTests: XCTestCase {
    private func item(_ sequence: Int, _ folded: Int) -> MissionItem {
        MissionItem(index: sequence, sequence: sequence, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0, kind: "waypoint", foldedCommands: folded)
    }

    func testAnItemThatFoldsNothingIsNumberedPlainly() {
        XCTAssertEqual("2", sequenceLabel(item(2, 0)))
    }

    func testAnItemThatFoldsACommandOwnsTheSpanSoTheNextNumberIsNotAHole() {
        XCTAssertEqual("2–3", sequenceLabel(item(2, 1)))
    }

    func testASurveyOwnsEverySequenceItGenerates() {
        XCTAssertEqual("4–216", sequenceLabel(item(4, 212)))
    }

    func testTheRowsOfAPlanWithAFoldedSpeedLeaveNoGapBetweenThem() {
        XCTAssertEqual(["0", "1", "2–3", "4"], itemRows([item(0, 0), item(1, 0), item(2, 1), item(4, 0)]).map(\.number))
    }

    func testAnItemTheCoreWithheldTheSpanForIsNumberedPlainlyRatherThanGivenARange() {
        let withheld = #"{"kind":"object","items":[{"index":0,"sequence":2,"name":"Waypoint","kind":"waypoint","foldedCommands":null}]}"#
        let absent = #"{"kind":"object","items":[{"index":0,"sequence":2,"name":"Waypoint","kind":"waypoint"}]}"#
        let folded = #"{"kind":"object","items":[{"index":0,"sequence":2,"name":"Waypoint","kind":"waypoint","foldedCommands":1}]}"#
        XCTAssertEqual(["2"], allMissionItems(JSON.parse(withheld)).map(sequenceLabel))
        XCTAssertEqual(["2"], allMissionItems(JSON.parse(absent)).map(sequenceLabel))
        XCTAssertEqual(["2\u{2013}3"], allMissionItems(JSON.parse(folded)).map(sequenceLabel))
    }
}
