import XCTest
@testable import Aircast

final class FenceListTests: XCTestCase {
    func testARowSelectsItsShapeAndADeletedRallyPointHandsSelectionToItsNeighbour() {
        let polygon = FenceRow(index: 1, circle: false, title: "Polygon 2", detail: "", inclusion: false)
        let circle = FenceRow(index: 0, circle: true, title: "Circle 1", detail: "")
        XCTAssertEqual(MapHit.FenceVertex(polygon: 1, vertex: 0), fenceRowHit(polygon))
        XCTAssertEqual(MapHit.Circle(index: 0), fenceRowHit(circle))
        XCTAssertTrue(rowSelected(polygon, .FenceVertex(polygon: 1, vertex: 3)))
        XCTAssertEqual(MapHit.Rally(index: 1), rallyAfterRemove(1, 3), "RallyPointController selects the next point")
        XCTAssertEqual(MapHit.Rally(index: 1), rallyAfterRemove(2, 3), "or the new last one")
        XCTAssertNil(rallyAfterRemove(0, 1))
    }

    func testPolygonAndCircleRowsSitUnderGeoFenceEditorSectionLabels() {
        let polygons = [FenceRow(index: 0, circle: false, title: "a", detail: ""), FenceRow(index: 1, circle: false, title: "b", detail: "")]
        let circle = FenceRow(index: 0, circle: true, title: "c", detail: "")
        XCTAssertEqual("Polygon fences", fenceHeading(polygons[0], nil))
        XCTAssertNil(fenceHeading(polygons[1], polygons[0]))
        XCTAssertEqual("Circular fences", fenceHeading(circle, polygons[1]))
        XCTAssertEqual("Circular fences", fenceHeading(circle, nil))
    }
}
