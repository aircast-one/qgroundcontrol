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
        let numbered = { (index: Int, sequence: Int, stop: Int?, folded: Int) in
            MissionItem(index: index, sequence: sequence, stop: stop, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0, kind: "waypoint", foldedCommands: folded)
        }
        XCTAssertEqual(["H", "1", "2", ""], itemRows([numbered(0, 0, nil, 0), numbered(1, 1, 1, 0), numbered(2, 2, 2, 1), numbered(3, 4, nil, 0)]).map(\.number), "waypoints are numbered 1, 2, 3 whatever commands fold into them, and an action under a waypoint has no number of its own")
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
