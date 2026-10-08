import XCTest
@testable import Aircast

final class FenceRowsTests: XCTestCase {
    func testPolygonsThenCirclesNamedLikeGeoFenceEditorRowsWithInclusionOrExclusionBeneath() {
        let polygon = FencePolygon(index: 0, inclusion: false, vertices: [], detailText: "4 corners")
        let circle = FenceCircle(index: 0, inclusion: true, centre: TrackPoint(0.0, 0.0), radius: 137.0, detailText: "137 m radius", kindText: "Circle 1", radiusUnits: "m")
        XCTAssertEqual(
            [FenceRow(index: 0, circle: false, title: "Polygon 1", detail: "Exclusion", inclusion: false), FenceRow(index: 0, circle: true, title: "Circle 1", detail: "Inclusion", radius: "137 m")],
            fenceRows([polygon], [circle])
        )
    }

    func testRallyPointsAreNumberedFromOneWithTheirHeightAndPosition() {
        let point = RallyPoint(index: 1, latitude: 41.7151, longitude: 44.8271, altitude: 50.0, altitudeUnits: "m")
        XCTAssertEqual([FenceRow(index: 1, circle: false, title: "Rally 2", detail: "50 m \u{00b7} 41.715100, 44.827100")], rallyRows([point]))
        var unknown = point
        unknown.altitude = .nan
        XCTAssertEqual(["41.715100, 44.827100"], rallyRows([unknown]).map(\.detail))
    }

    func testSelectingAFenceOrRallyShapeOpensItsLayerAWaypointTheMissions() {
        XCTAssertEqual(PlanLayer.Fence, layerOf(.Circle(index: 0)))
        XCTAssertEqual(PlanLayer.Fence, layerOf(.FenceVertex(polygon: 0, vertex: 1)))
        XCTAssertEqual(PlanLayer.Rally, layerOf(.Rally(index: 0)))
        XCTAssertEqual(PlanLayer.Mission, layerOf(.Waypoint(index: 2)))
        XCTAssertEqual(PlanLayer.Mission, layerOf(.ShapeCentre(fence: false, owner: 3)))
        XCTAssertNil(layerOf(nil))
    }
}
