import XCTest
@testable import Aircast

final class CorridorAreaTests: XCTestCase {
    func testACorridorShadesItsWidthOutlineLikeTransectStyleMapVisuals() {
        let point = { (lat: Double) in TrackPoint(latitude: lat, longitude: 8.0) }
        let corridor = Survey(index: 1, area: [point(47.0), point(47.1)], transects: [], cameraShots: 0, kind: "corridor", shape: SHAPE_LINE, property: "corridorPolyline", outline: [point(47.0), point(47.05), point(47.1)])
        let survey = Survey(index: 2, area: [point(46.0), point(46.1), point(46.2)], transects: [], cameraShots: 0, kind: "survey", shape: SHAPE_AREA, property: "surveyAreaPolygon")
        XCTAssertEqual(shadedArea(corridor), corridor.outline)
        XCTAssertEqual(shadedArea(survey), survey.area)
        XCTAssertEqual(surveyAreaFeatures([corridor, survey]).shapes.count, 2)
        let tinted = surveyAreaFeatures([withChanges(survey) { $0.collides = true }]).shapes
        XCTAssertEqual(tinted.count, 1)
        XCTAssertEqual(tinted.first?.getBooleanProperty(SURVEY_COLLISION), true, "a pattern that clips terrain is drawn in surveyPolygonTerrainCollision")
        XCTAssertEqual(collidingItems(JSON.parse(#"{"collidingItems":[2]}"#)), [2])
    }
}
