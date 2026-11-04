#if os(macOS)
import XCTest
@testable import Aircast

final class MapContextMenuTests: XCTestCase {
    private func waypoint(_ index: Int, _ longitude: Double) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: 0, longitude: longitude, command: "Waypoint", selected: false, kind: KIND_WAYPOINT, commandId: 16)
    }

    private func choices(_ lines: [PlanMenuLine]) -> [PlanMenuChoice?] { lines.map(\.choice) }

    func testAWaypointOffersEditInsertAfterAndDeleteUnderItsName() {
        let lines = planMenuLines(.Waypoint(index: 2), PlanMenuOffer(layer: .Mission, items: [waypoint(1, 0), waypoint(2, 0.001)], itemCount: 2))
        XCTAssertEqual(choices(lines), [nil, .Edit(.Waypoint(index: 2)), .InsertWaypointAfter(index: 2), .DeleteItem(index: 2)])
        XCTAssertFalse(lines[0].enabled, "the item's name is a caption, not a command")
        XCTAssertEqual(lines.map(\.startsGroup), [false, true, false, true])
    }

    func testHomeCannotBeDeletedFromTheMenuAsThePanelOffersNoRemoveForIt() {
        let lines = planMenuLines(.Waypoint(index: HOME_ITEM), PlanMenuOffer(layer: .Mission))
        XCTAssertFalse(choices(lines).contains(.DeleteItem(index: HOME_ITEM)))
        XCTAssertTrue(choices(lines).contains(.InsertWaypointAfter(index: HOME_ITEM)))
    }

    func testAHandleEditsTheItemOrCircleItBelongsTo() {
        XCTAssertEqual(choices(planMenuLines(.LoiterRadius(index: 3), PlanMenuOffer(layer: .Mission)))[1], .Edit(.Waypoint(index: 3)))
        XCTAssertEqual(
            choices(planMenuLines(.CircleRadius(index: 1), PlanMenuOffer(layer: .Fence))),
            [nil, .Edit(.Circle(index: 1)), .DeleteCircle(index: 1)]
        )
    }

    func testAFenceCornerOffersRemoveVertexOnlyWhenThePolygonCanLoseOne() {
        let corner = MapHit.FenceVertex(polygon: 0, vertex: 2)
        XCTAssertEqual(
            choices(planMenuLines(corner, PlanMenuOffer(layer: .Fence, cornerRemovable: true))),
            [nil, .Edit(corner), .RemoveVertex(corner), .DeleteFence(polygon: 0)]
        )
        XCTAssertEqual(choices(planMenuLines(corner, PlanMenuOffer(layer: .Fence))), [nil, .Edit(corner), .DeleteFence(polygon: 0)])
        let surveyCorner = MapHit.SurveyVertex(item: 4, vertex: 1)
        XCTAssertTrue(choices(planMenuLines(surveyCorner, PlanMenuOffer(layer: .Mission, cornerRemovable: true))).contains(.RemoveVertex(surveyCorner)))
    }

    func testARallyPointAndTheBreachReturnOfferWhatTheirPanelsDo() {
        let rally = planMenuLines(.Rally(index: 1), PlanMenuOffer(layer: .Rally))
        XCTAssertEqual(choices(rally), [nil, .Edit(.Rally(index: 1)), .DeleteRally(index: 1)])
        XCTAssertEqual(rally[0].title, "Rally point 2")
        XCTAssertEqual(choices(planMenuLines(.BreachReturn, PlanMenuOffer(layer: .Fence))), [nil, .Edit(.BreachReturn)])
    }

    func testALegsSplitHandleInsertsAWaypointAndAShapesAddsACorner() {
        let leg = MapHit.Midpoint(path: MISSION_SPLIT_PATH, invokable: MISSION_SPLIT_INVOKABLE, segment: 2)
        XCTAssertEqual(planMenuLines(leg, PlanMenuOffer(layer: .Mission)), [PlanMenuLine(choice: .Split(leg), title: "Insert waypoint here")])
        let edge = MapHit.Midpoint(path: "\(FENCE_POLYGONS).0", invokable: "split", segment: 1)
        XCTAssertEqual(planMenuLines(edge, PlanMenuOffer(layer: .Fence)).map(\.title), ["Add corner here"])
    }

    func testBlankMissionMapOffersTheRailsAddKindsAtThePoint() {
        let kinds = [
            MissionKind(id: KIND_WAYPOINT, label: "", title: "Waypoint", enabled: true, disabledReason: ""),
            MissionKind(id: KIND_TAKEOFF, label: "", title: "Takeoff", enabled: true, disabledReason: ""),
            MissionKind(id: KIND_LAND, label: "", title: "Land", enabled: true, disabledReason: ""),
            MissionKind(id: KIND_SURVEY, label: "Survey", title: "Survey", enabled: true, disabledReason: ""),
        ]
        let empty = planMenuLines(nil, PlanMenuOffer(layer: .Mission, kinds: kinds, itemCount: 0, takeoffMissing: true))
        XCTAssertEqual(choices(empty), [.AddWaypoint, .AddTakeoff, .AddLanding, .AddPattern])
        XCTAssertEqual(empty.map(\.enabled), [true, true, false, true], "a landing needs an item to follow, as the rail's Land does")
        let started = planMenuLines(nil, PlanMenuOffer(layer: .Mission, kinds: kinds, itemCount: 3, takeoffMissing: false))
        XCTAssertEqual(choices(started), [.AddWaypoint, .AddLanding, .AddPattern])
    }

    func testBlankFenceAndRallyMapsOfferTheirRailsAdds() {
        XCTAssertEqual(
            choices(planMenuLines(nil, PlanMenuOffer(layer: .Fence, fence: true))),
            [.AddPolygonFence, .AddCircleFence, .SetBreachReturn]
        )
        XCTAssertEqual(planMenuLines(nil, PlanMenuOffer(layer: .Fence, fence: true, breachSet: true)).last?.title, "Move breach return here")
        XCTAssertTrue(planMenuLines(nil, PlanMenuOffer(layer: .Fence, fenceRefused: true)).isEmpty)
        XCTAssertEqual(planMenuLines(nil, PlanMenuOffer(layer: .Rally)), [PlanMenuLine(choice: .AddRally, title: "Add rally point here", enabled: false)])
    }

    func testTracingOnlyAddsTracePoints() {
        XCTAssertEqual(choices(planMenuLines(nil, PlanMenuOffer(layer: .Mission, tracing: true))), [.AddTracePoint])
    }

    func testInsertAfterPutsTheWaypointHalfwayAlongTheNextLegOrWaitsAfterTheLastItem() {
        let items = [waypoint(1, 0), waypoint(2, 0.002)]
        let between = insertAfterPoint(1, items)
        XCTAssertEqual(between?.latitude ?? .nan, 0, accuracy: 1e-9)
        XCTAssertEqual(between?.longitude ?? .nan, 0.001, accuracy: 1e-6)
        XCTAssertNil(insertAfterPoint(2, items))
    }

    func testTheFlyMenuListsTheSheetsActionsAndMarksTheOnesThatOpenTheSheet() {
        let goTo = MapClickAction(id: GOTO_ACTION, path: "vehicle.guidedModeGotoLocation", label: "Go to location", title: "Go to location", message: "", confirm: true)
        let orbit = MapClickAction(id: ORBIT_ACTION, path: "vehicle.guidedModeOrbit", label: "Orbit at location", title: "Orbit", message: "", confirm: false)
        let roi = MapClickAction(id: "Roi", path: "vehicle.guidedModeROI", label: "ROI at location", title: "ROI", message: "", confirm: false)
        let choices = flyMenuChoices([goTo, orbit, roi], 4)
        XCTAssertEqual(choices, [.SetWaypoint(sequence: 4), .Action(goTo), .Action(orbit), .Action(roi)])
        XCTAssertEqual(choices.map(flyMenuTitle), ["Set current waypoint to 4\u{2026}", "Go to location\u{2026}", "Orbit at location\u{2026}", "ROI at location"])
        XCTAssertTrue(opensClickSheet(goTo))
        XCTAssertTrue(opensClickSheet(orbit), "an orbit always asks its radius and height first")
        XCTAssertFalse(opensClickSheet(roi))
        XCTAssertEqual(flyMenuChoices([], nil), [])
    }

    func testTheMenuNeverSendsAnActionTheSheetWouldConfirm() {
        let goTo = MapClickAction(id: GOTO_ACTION, path: "vehicle.guidedModeGotoLocation", label: "Go to location", title: "Go to location", message: "", confirm: true)
        XCTAssertNotNil(sendMapClick(goTo, MapPoint(latitude: 47.4, longitude: 8.5)))
    }

    func testOnlyAWaypointHitOffersSettingTheCurrentWaypoint() {
        let items = [waypoint(1, 0), waypoint(2, 0.001)]
        XCTAssertEqual(flyMenuWaypoint(.Waypoint(index: 2), items), 2)
        XCTAssertNil(flyMenuWaypoint(.Rally(index: 0), items))
        XCTAssertNil(flyMenuWaypoint(nil, items))
    }
}
#endif
