import XCTest
@testable import Aircast

final class DropMarkerTests: XCTestCase {
    private func waypoint(_ index: Int) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: 47.0 + Double(index), longitude: 8.0, command: "Waypoint", selected: false, commandId: 16, specifiesCoordinate: true)
    }

    private func unplaced(_ index: Int, _ command: Int, drops: Bool = false) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: .nan, longitude: .nan, command: "", selected: false, commandId: command, placed: false, dropsPayload: drops)
    }

    func testAWaypointFollowedByAGripperReleaseIsMarkedAsTheDropPoint() {
        let items = [waypoint(1), unplaced(2, 211, drops: true), waypoint(3), unplaced(4, 178), waypoint(5)]
        XCTAssertEqual(items.map { dropsAt(items, $0) }, [true, false, false, false, false])
        let sides = Dictionary(uniqueKeysWithValues: missionFeatures(withDrops(items).filter(\.placed)).shapes.map { ($0.getNumberProperty(WAYPOINT_ID_PROPERTY).map(Int.init) ?? -1, $0.getStringProperty(WAYPOINT_SIDE_LABEL_PROPERTY) ?? "") })
        XCTAssertEqual(sides[1], DROP_SIDE)
        XCTAssertEqual(sides[3], "")
    }

    func testThePlanMapReadMarksTheWaypointBeforeADropThoughTheDropItselfHasNoPlace() {
        let plan = allMissionItems(JSON.parse(#"{"kind":"object","items":[{"sequence":1,"name":"Waypoint","command":16,"specifiesCoordinate":true,"coordinate":{"latitude":47.0,"longitude":8.0}},{"sequence":2,"name":"Drop payload","command":211,"dropsPayload":true}]}"#))
        XCTAssertEqual(plan.filter(\.placed).map(\.drops), [true])
    }

    func testABayServoDropMarksItsWaypointAndAServoThatDropsNothingDoesNot() {
        let items = [waypoint(1), unplaced(2, 183, drops: true), waypoint(3), unplaced(4, 183), waypoint(5)]
        XCTAssertEqual(items.map { dropsAt(items, $0) }, [true, false, false, false, false])
    }
}
