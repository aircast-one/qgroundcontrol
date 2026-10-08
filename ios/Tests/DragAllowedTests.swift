import XCTest
@testable import Aircast

final class DragAllowedTests: XCTestCase {
    func testOnlyTheSelectedItemOnTheActiveLayerCanBeDragged() {
        XCTAssertTrue(dragAllowed(.Waypoint(index: 2), .Waypoint(index: 2), .Mission))
        XCTAssertFalse(dragAllowed(.Waypoint(index: 3), .Waypoint(index: 2), .Mission), "MissionItemMapVisualBase only drags the current item")
        XCTAssertTrue(dragAllowed(.SurveyVertex(item: 2, vertex: 1), .Waypoint(index: 2), .Mission), "a selected survey's corners move")
        XCTAssertFalse(dragAllowed(.Rally(index: 0), .Rally(index: 0), .Mission), "a rally point is not dragged from the mission layer")
        XCTAssertTrue(dragAllowed(.Rally(index: 0), .Rally(index: 0), .Rally))
        XCTAssertFalse(dragAllowed(.FenceVertex(polygon: 0, vertex: 1), nil, .Fence), "nothing selected, nothing dragged")
        XCTAssertTrue(dragAllowed(.FenceVertex(polygon: 0, vertex: 1), .FenceVertex(polygon: 0, vertex: 0), .Fence))
        XCTAssertTrue(dragAllowed(.ShapeRadius(fence: true, owner: 1), .FenceVertex(polygon: 1, vertex: 0), .Fence), "a selected polygon's radius handle is its own")
    }

    func testAMidpointSplitsOnlyTheSelectedShape() {
        XCTAssertTrue(dragAllowed(.Midpoint(path: "\(FENCE_POLYGONS).1", invokable: "split", segment: 0), .FenceVertex(polygon: 1, vertex: 0), .Fence))
        XCTAssertFalse(dragAllowed(.Midpoint(path: "\(FENCE_POLYGONS).1", invokable: "split", segment: 0), .FenceVertex(polygon: 0, vertex: 0), .Fence))
        XCTAssertTrue(dragAllowed(.Midpoint(path: "\(PLAN_ITEMS).2.surveyAreaPolygon", invokable: "split", segment: 0), .Waypoint(index: 2), .Mission))
        XCTAssertFalse(dragAllowed(.Midpoint(path: "\(PLAN_ITEMS).2.surveyAreaPolygon", invokable: "split", segment: 0), .Waypoint(index: 3), .Mission))
        XCTAssertTrue(dragAllowed(.Midpoint(path: MISSION_SPLIT_PATH, invokable: MISSION_SPLIT_INVOKABLE, segment: 2), .Waypoint(index: 2), .Mission))
    }
}
