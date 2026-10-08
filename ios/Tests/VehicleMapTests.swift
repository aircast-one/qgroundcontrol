import MapLibre
import XCTest
@testable import Aircast

final class VehicleMapTests: XCTestCase, MLNMapViewDelegate {
    private var loaded: XCTestExpectation?
    private var style: MLNStyle?

    func mapView(_ mapView: MLNMapView, didFinishLoading style: MLNStyle) {
        self.style = style
        loaded?.fulfill()
    }

    func testEveryLayerInstallsAndEveryRendererDrawsOnARealStyle() throws {
        let file = FileManager.default.temporaryDirectory.appendingPathComponent("vehicle-map-test-style.json")
        try OSM_RASTER_STYLE.write(to: file, atomically: true, encoding: .utf8)
        let map = MLNMapView(frame: CGRect(x: 0, y: 0, width: 300, height: 600), styleURL: file)
        map.delegate = self
        let waiting = expectation(description: "style")
        loaded = waiting
        wait(for: [waiting], timeout: 30)
        let style = try XCTUnwrap(style)
        VehicleMapModel().mapView(map, didFinishLoading: style)
        [MISSION_DOT_LAYER, MISSION_PATH_LAYER, FENCE_FILL_LAYER, RALLY_LAYER, GCS_LAYER, BREACH_LAYER, FENCE_HANDLE_LAYER, MIDPOINT_LAYER,
         SURVEY_AREA_LAYER, LANDING_AREA_LAYER, EDGE_LABEL_LAYER, SHOT_LAYER, TRAFFIC_LAYER, ROI_LAYER].forEach {
            XCTAssertNotNil(style.layer(withIdentifier: $0), $0)
        }
        let at = TrackPoint(latitude: 47, longitude: 8)
        let items = (0..<5).map {
            MissionItem(index: $0, sequence: $0, latitude: 47 + Double($0) * 0.001, longitude: 8, command: "Waypoint", selected: false, kind: "waypoint", loiterRadius: $0 == 3 ? 50 : .nan, heading: 90, gimbalYaw: 10)
        }
        let fence = FencePolygon(index: 0, inclusion: true, vertices: [at, TrackPoint(latitude: 47.01, longitude: 8), TrackPoint(latitude: 47.01, longitude: 8.01)])
        let circle = FenceCircle(index: 0, inclusion: false, centre: at, radius: 100)
        let survey = Survey(index: 2, area: fence.vertices, transects: fence.vertices, cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon", turnaround: true)
        let landing = LandingPattern(index: 4, landing: at, slopeStart: TrackPoint(latitude: 47.01, longitude: 8), finalApproach: TrackPoint(latitude: 47.02, longitude: 8), loiterRadiusMetres: 80, loiterClockwise: true, loiterToAltitude: true)
        renderSurveys(style, [survey])
        renderTransectMarks(style, [survey], 2, legArrows: legArrows(items, true) + loiterRotationArrows(items))
        renderGimbalWedges(style, items)
        renderMidpoints(style, [fence], [survey], items: items, selected: 2)
        renderFences(style, [fence], [RallyPoint(index: 0, latitude: 47, longitude: 8)], circles: circlesAsPolygons([circle]), breach: at)
        style.setGeoJson(GCS_SOURCE, operatorFeatures(at, heading: 45))
        renderVertexHandles(style, [fence], [survey], circles: [circle], landings: [landing], loiterHandles: loiterHandleFeatures(items, 3))
        renderMission(style, items, true, selectedIndex: 1, others: [OtherMission(items: items, linkStartToHome: false)], landings: [landing])
        renderCollisionLegs(style, [(at, TrackPoint(latitude: 47.01, longitude: 8))])
        renderGoto(style, GotoLocation(at: at, loiterRadiusMetres: 60, loiterRadiusText: "60 m"), editing: true)
        renderOrbit(style, OrbitCircle(centre: at, radiusMetres: 50), false)
        renderRoi(style, at)
        renderClickMarker(style, at)
        renderGimbals(style, 47, 8, [GimbalAzimuth(yaw: 20, active: true)])
        renderProximityRadars(style, [PlacedRadar(at: at, heading: 0, reading: RadarReading(maxMeters: 40, sectors: [(0, 10)]))])
        style.setGeoJson(TRAFFIC_SOURCE, trafficFeatures([TrafficMark(latitude: 47, longitude: 8, heading: 90, alert: true, label: "x")]))
        style.setGeoJson(SHOT_SOURCE, shotFeatures([at]))
    }

    func testARallyPointWithNoAltitudeStillEqualsItselfSoThePlanLayersDoNotRedrawEveryFrame() {
        let rally = RallyPoint(index: 0, latitude: 47, longitude: 8)
        XCTAssertEqual(rally, rally)
        XCTAssertNotEqual(rally, withChanges(rally) { $0.altitude = 30 })
    }
}
