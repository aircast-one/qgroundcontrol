import XCTest
@testable import Aircast

final class CenterMenuTests: XCTestCase {
    func testATypedCoordinateCentresOnlyWhenItIsOnTheGlobe() {
        XCTAssertEqual(TrackPoint(47.5, 8.25), parsedCoordinate(" 47.5 ", "8.25"))
        XCTAssertNil(parsedCoordinate("91", "8"))
        XCTAssertNil(parsedCoordinate("47", "east"))
    }

    func testMissionFitUsesTheMissionItemsThatHaveAPosition() {
        let items = [
            MissionItem(index: 0, sequence: 0, latitude: 47.0, longitude: 8.0, command: "Home", selected: false),
            MissionItem(index: 1, sequence: 1, latitude: .nan, longitude: .nan, command: "Delay", selected: false),
            MissionItem(index: 2, sequence: 2, latitude: 47.1, longitude: 8.1, command: "Waypoint", selected: false),
        ]
        XCTAssertEqual([TrackPoint(47.0, 8.0), TrackPoint(47.1, 8.1)], missionFitPoints(items))
    }

    func testASelectedCornersPositionComesFromItsFenceOrSurvey() {
        let fence = FencePolygon(index: 3, inclusion: true, vertices: [TrackPoint(1.0, 2.0), TrackPoint(3.0, 4.0)])
        let survey = Survey(index: 5, area: [TrackPoint(5.0, 6.0)], transects: [], cameraShots: 0, kind: "survey", shape: "polygon", property: "surveyAreaPolygon")
        XCTAssertEqual(TrackPoint(3.0, 4.0), cornerPosition(.FenceVertex(polygon: 3, vertex: 1), [fence], [survey]))
        XCTAssertEqual(TrackPoint(5.0, 6.0), cornerPosition(.SurveyVertex(item: 5, vertex: 0), [fence], [survey]))
        XCTAssertNil(cornerPosition(.FenceVertex(polygon: 3, vertex: 9), [fence], [survey]))
    }
}
