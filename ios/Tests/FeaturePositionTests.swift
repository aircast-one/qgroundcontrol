import MapLibre
import XCTest
@testable import Aircast

final class FeaturePositionTests: XCTestCase {
    private func at(_ latitude: Double, _ longitude: Double) -> TrackPoint { TrackPoint(latitude: latitude, longitude: longitude) }

    private func points(_ features: FeatureCollection) -> [TrackPoint] {
        features.shapes.compactMap { ($0 as? MLNPointFeature).map { TrackPoint($0.coordinate) } }
    }

    private func ring(_ features: FeatureCollection) -> [TrackPoint] {
        (features.shapes.first as? MLNPolygonFeature)?.trackPoints ?? []
    }

    private func line(_ features: FeatureCollection) -> [TrackPoint] {
        (features.shapes.first as? MLNPolylineFeature)?.trackPoints ?? []
    }

    private func item(_ index: Int, _ latitude: Double, _ longitude: Double) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: latitude, longitude: longitude, command: "Waypoint", selected: false)
    }

    func testAWaypointMarkerSitsOnTheWaypoint() {
        XCTAssertEqual(points(missionFeatures([item(0, 41.0, 44.0), item(1, 41.2, 44.3)])), [at(41.0, 44.0), at(41.2, 44.3)])
    }

    func testARallyMarkerSitsOnTheRallyPoint() {
        XCTAssertEqual(points(rallyFeatures([RallyPoint(index: 0, latitude: 41.5, longitude: 44.5)])), [at(41.5, 44.5)])
    }

    func testTheVehicleMarkerSitsOnTheVehicle() {
        XCTAssertEqual(points(vehicleFeatures(41.7, 44.8, 90.0)), [at(41.7, 44.8)])
    }

    func testAFenceRingRunsThroughItsVerticesAndCloses() {
        let fence = FencePolygon(index: 0, inclusion: true, vertices: [at(41.0, 44.0), at(41.0, 44.1), at(41.1, 44.1)])
        XCTAssertEqual(ring(fenceFeatures([fence])), [at(41.0, 44.0), at(41.0, 44.1), at(41.1, 44.1), at(41.0, 44.0)])
    }

    func testASurveyAreaRunsThroughItsCornersAndCloses() {
        let survey = Survey(index: 0, area: [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)], transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon")
        XCTAssertEqual(ring(surveyAreaFeatures([survey])), [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1), at(42.0, 45.0)])
    }

    func testACorridorIsDrawnOpenBecauseItsPathIsNotABoundary() {
        let corridor = Survey(index: 0, area: [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)], transects: [], cameraShots: 0, kind: "corridor", shape: SHAPE_LINE, property: "corridorPolyline")
        XCTAssertEqual(line(surveyLineFeatures([corridor])), [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)])
        XCTAssertEqual(surveyAreaFeatures([corridor]).shapes.count, 0)
    }

    func testAStructureScanIsDrawnClosedLikeASurvey() {
        let structure = Survey(index: 0, area: [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)], transects: [], cameraShots: 0, kind: "structure", shape: SHAPE_AREA, property: "structurePolygon")
        XCTAssertEqual(ring(surveyAreaFeatures([structure])), [at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1), at(42.0, 45.0)])
        XCTAssertEqual(surveyLineFeatures([structure]).shapes.count, 0)
    }

    func testSurveyTransectsRunThroughTheirPointsInOrder() {
        let survey = Survey(index: 0, area: [], transects: [at(43.0, 46.0), at(43.1, 46.1)], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon")
        XCTAssertEqual(line(surveyTransectFeatures([survey])), [at(43.0, 46.0), at(43.1, 46.1)])
    }
}
