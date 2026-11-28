import XCTest
@testable import Aircast

final class ActionOrderTests: XCTestCase {
    private func steps(_ hoverPlaces: String = "[0,1,2,3]", _ otherPlaces: String = "[0,1,2,3]") -> [ActionStep] {
        actionSteps(JSON.parse(#"""
        {"actions":[
        {"kind":"Hover","title":"Hover","seconds":10,"item":null,"places":\#(hoverPlaces),"refusal":"stays first"},
        {"kind":"Gimbal","title":"Gimbal","seconds":null,"item":null,"places":\#(otherPlaces),"refusal":null},
        {"kind":"Camera","title":"Camera","seconds":null,"item":null,"places":\#(otherPlaces),"refusal":null},
        {"kind":"Command","title":"Set servo","seconds":null,"item":5,"places":\#(otherPlaces),"refusal":null}]}
        """#))
    }

    private let active = [
        WaypointAction(kind: .Hover, title: "Hover", value: "10 s", icon: .planTimer),
        WaypointAction(kind: .Gimbal, title: "Gimbal", value: "0° / 60°", icon: .planGimbal),
        WaypointAction(kind: .Camera, title: "Take photo", value: "", icon: .planPhoto),
        WaypointAction(kind: .Turn, title: "Turn aircraft", value: "90°", icon: .planTurn),
    ]

    func testTheRowsReadTheCoresOrderAndTurningTheAircraftStaysLastWithNowhereToDrag() {
        let ordered = orderedActions(steps(), active)
        XCTAssertEqual(ordered.map(\.title), ["Hover", "Gimbal", "Take photo", "Set servo", "Turn aircraft"])
        XCTAssertEqual(ordered.map(\.place), [0, 1, 2, 3, nil])
        XCTAssertFalse(canDrop(ordered.last, 0))
        XCTAssertEqual(steps().first?.places, [0, 1, 2, 3])
        XCTAssertEqual(actionSteps(nil), [])
    }

    func testASlotTheAircraftCannotFlyOpensNothingAndLettingGoThereSendsNothing() {
        let ordered = orderedActions(steps("[0]", "[1,2,3]"), active).filter { $0.place != nil }
        XCTAssertEqual(projectedPlaces(ordered, 3, 0), [0, 1, 2, 3])
        XCTAssertNil(dropMove(2, ordered[3], 0))
        XCTAssertEqual(projectedPlaces(ordered, 3, 1), [0, 2, 3, 1])
        XCTAssertEqual(dropMove(2, ordered[3], 1), ActionMove(waypoint: 2, from: 3, to: 1))
        XCTAssertNil(dropMove(2, ordered[3], 3))
        XCTAssertNil(dropMove(2, ordered[3], nil))
    }

    func testTheHoldReadsAsAWaitAsSoonAsItWouldLeaveFirstPlace() {
        let ordered = orderedActions(steps(), active).filter { $0.place != nil }
        XCTAssertEqual([0, 2].map { shownAt(ordered[0], $0) }.map { "\($0.title) \($0.icon)" }, ["Hover planTimer", "Wait pause"])
        let projected = projectedPlaces(ordered, 2, 0)
        XCTAssertEqual(projected, [1, 2, 0, 3])
        XCTAssertEqual(shownAt(ordered[0], projected[0]).title, WAIT)
        XCTAssertEqual(shownAt(ordered[1], 3).title, "Gimbal")
    }

    func testADragThroughEverySlotSendsOneMoveForThePlaceItWasLetGo() {
        let ordered = orderedActions(steps(), active).filter { $0.place != nil }
        let centres: [CGFloat] = [20, 70, 120, 170]
        let dragged = [-30, -60, -90, -120, -150].map { ReorderDrag(lifted: 3).moved(to: CGFloat($0), centres, top: 0, bottom: 190, margin: 32) }
        XCTAssertEqual(dragged.map(\.over), [2, 2, 1, 1, 0])
        XCTAssertEqual(dragged.last?.released(2, ordered), ActionMove(waypoint: 2, from: 3, to: 0))
        XCTAssertNil(ReorderDrag(lifted: 3).moved(to: 400, centres, top: 0, bottom: 190, margin: 32).released(2, ordered))
        XCTAssertEqual(movedOrder([10, 20, 1], 0, 2), [20, 1, 10])
    }

    func testRowsReadBeforeThePlanLastChangedAreNotActedOn() {
        XCTAssertEqual(planRevision(JSON.parse(#"{"revision":5}"#)), 5)
        XCTAssertEqual(planRevision(nil), NO_REVISION)
        XCTAssertEqual([(6, 5), (5, 5), (4, 5)].map { rowsAreStale($0.0, $0.1) }, [true, false, false])
    }

    func testATapHeldWhileTheRowsWereStalePicksTheItemThePilotSaw() {
        let waypoint = { (index: Int, latitude: Double) in MissionItem(index: index, sequence: index, latitude: latitude, longitude: 8, command: "Waypoint", selected: false, commandId: 16) }
        let delay = { (index: Int) in MissionItem(index: index, sequence: index, latitude: .nan, longitude: .nan, command: "Delay", selected: false, commandId: 93) }
        XCTAssertEqual(sameItemIn(waypoint(3, 47.2), [waypoint(1, 47.1), delay(2), delay(3), waypoint(4, 47.2)])?.index, 4)
        XCTAssertEqual(sameItemIn(delay(3), [waypoint(1, 47.1), delay(2), delay(4)])?.index, 2, "of two alike, the nearest to where it was")
        XCTAssertNil(sameItemIn(waypoint(3, 47.2), [waypoint(1, 47.1)]), "gone since")
    }
}
