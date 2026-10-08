import MapLibre
import XCTest
@testable import Aircast

final class FenceHandleTests: XCTestCase {
    private func polygon(_ index: Int, _ vertices: (Double, Double)...) -> FencePolygon {
        FencePolygon(index: index, inclusion: true, vertices: vertices.map { TrackPoint(latitude: $0.0, longitude: $0.1) })
    }

    private func survey(_ index: Int, _ vertices: (Double, Double)...) -> Survey {
        Survey(index: index, area: vertices.map { TrackPoint(latitude: $0.0, longitude: $0.1) }, transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon")
    }

    private func placed(_ features: FeatureCollection) -> [TrackPoint] {
        features.shapes.compactMap { ($0 as? MLNPointFeature).map { TrackPoint($0.coordinate) } }
    }

    func testEveryVertexOfEveryPolygonGetsAHandle() {
        let features = vertexHandleFeatures(
            [polygon(0, (41.0, 44.0), (41.0, 44.1), (41.1, 44.1)), polygon(1, (42.0, 45.0), (42.0, 45.1), (42.1, 45.1), (42.1, 45.0))],
            []
        )
        XCTAssertEqual(features.shapes.count, 7)
    }

    func testAHandleSitsOnTheVertexItEditsLongitudeNotSwappedForLatitude() {
        let features = vertexHandleFeatures([polygon(0, (41.0, 44.0), (41.2, 44.3))], [])
        XCTAssertEqual(placed(features), [TrackPoint(latitude: 41.0, longitude: 44.0), TrackPoint(latitude: 41.2, longitude: 44.3)])
    }

    func testSurveyAndCircleHandlesArePlacedOnTheirOwnPointsToo() {
        let features = vertexHandleFeatures(
            [polygon(0, (41.0, 44.0))],
            [survey(1, (42.0, 45.0))],
            circles: [FenceCircle(index: 2, inclusion: true, centre: TrackPoint(latitude: 43.0, longitude: 46.0), radius: 150.0)]
        )
        XCTAssertEqual(
            Array(placed(features).prefix(3)),
            [TrackPoint(latitude: 41.0, longitude: 44.0), TrackPoint(latitude: 42.0, longitude: 45.0), TrackPoint(latitude: 43.0, longitude: 46.0)]
        )
    }

    func testAHandleCarriesThePolygonAndVertexItBelongsTo() {
        let second = vertexHandleFeatures([polygon(3, (41.0, 44.0), (41.0, 44.1), (41.1, 44.1))], []).shapes[1]
        XCTAssertEqual(second.getStringProperty(HANDLE_KIND_PROPERTY), HANDLE_KIND_FENCE)
        XCTAssertEqual(second.getNumberProperty(POLYGON_INDEX_PROPERTY).map { Int($0) }, 3)
        XCTAssertEqual(second.getNumberProperty(VERTEX_INDEX_PROPERTY).map { Int($0) }, 1)
    }

    func testASurveyAreaGetsHandlesMarkedAsItsOwnKind() {
        let features = vertexHandleFeatures([], [survey(5, (41.0, 44.0), (41.0, 44.1), (41.1, 44.1))]).shapes
        XCTAssertEqual(features.count, 3)
        XCTAssertEqual(features.first?.getStringProperty(HANDLE_KIND_PROPERTY), HANDLE_KIND_SURVEY)
        XCTAssertEqual(features.first?.getNumberProperty(POLYGON_INDEX_PROPERTY).map { Int($0) }, 5)
    }

    func testFenceAndSurveyHandlesShareOneCollectionWithoutColliding() {
        let features = vertexHandleFeatures(
            [polygon(0, (41.0, 44.0), (41.0, 44.1), (41.1, 44.1))],
            [survey(0, (42.0, 45.0), (42.0, 45.1), (42.1, 45.1))]
        ).shapes
        let kinds = features.map { $0.getStringProperty(HANDLE_KIND_PROPERTY) }
        XCTAssertEqual(features.count, 6)
        XCTAssertEqual(kinds.filter { $0 == HANDLE_KIND_FENCE }.count, 3)
        XCTAssertEqual(kinds.filter { $0 == HANDLE_KIND_SURVEY }.count, 3)
    }

    func testNothingToEditMeansNoHandles() {
        XCTAssertEqual(vertexHandleFeatures([], []).shapes.count, 0)
    }

    func testACircleGetsACentreHandleAndAnEdgeHandle() {
        let features = vertexHandleFeatures([], [], circles: [FenceCircle(index: 2, inclusion: true, centre: TrackPoint(latitude: 41.0, longitude: 44.0), radius: 150.0)]).shapes
        XCTAssertEqual(features.map { $0.getStringProperty(HANDLE_KIND_PROPERTY) }, [HANDLE_KIND_CIRCLE, HANDLE_KIND_FENCE_CIRCLE_RADIUS])
        XCTAssertEqual(features.first?.getNumberProperty(POLYGON_INDEX_PROPERTY).map { Int($0) }, 2)
        XCTAssertEqual(features.first?.getNumberProperty(VERTEX_INDEX_PROPERTY).map { Int($0) }, 0)
    }

    func testAllThreeKindsOfHandleCoexistAndStayDistinguishable() {
        let features = vertexHandleFeatures(
            [polygon(0, (41.0, 44.0), (41.0, 44.1), (41.1, 44.1))],
            [survey(0, (42.0, 45.0), (42.0, 45.1), (42.1, 45.1))],
            circles: [FenceCircle(index: 0, inclusion: true, centre: TrackPoint(latitude: 43.0, longitude: 46.0), radius: 150.0)]
        ).shapes
        let kinds = features.map { $0.getStringProperty(HANDLE_KIND_PROPERTY) }
        XCTAssertEqual(features.count, 8)
        XCTAssertEqual(kinds.filter { $0 == HANDLE_KIND_FENCE }.count, 3)
        XCTAssertEqual(kinds.filter { $0 == HANDLE_KIND_SURVEY }.count, 3)
        XCTAssertEqual(kinds.filter { $0 == HANDLE_KIND_CIRCLE }.count, 1)
        XCTAssertEqual(kinds.filter { $0 == HANDLE_KIND_FENCE_CIRCLE_RADIUS }.count, 1)
    }
}
