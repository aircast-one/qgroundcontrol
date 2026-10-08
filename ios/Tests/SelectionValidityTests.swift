import XCTest
@testable import Aircast

final class SelectionValidityTests: XCTestCase {
    private func item(_ index: Int) -> MissionItem {
        MissionItem(index: index, sequence: index + 1, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: .nan)
    }

    private func survives(_ selected: MapHit?, _ items: [MissionItem] = []) -> Bool {
        selectionSurvives(selected, items, [], [], [], [])
    }

    func testNoSelectionAlwaysSurvives() {
        XCTAssertTrue(survives(nil))
    }

    func testAWaypointThatIsStillThereSurvives() {
        XCTAssertTrue(survives(.Waypoint(index: 2), [item(1), item(2)]))
    }

    func testAWaypointThePlanNoLongerHoldsDoesNot() {
        XCTAssertFalse(survives(.Waypoint(index: 2), [item(0), item(1)]))
        XCTAssertFalse(survives(.Waypoint(index: 0), []))
    }

    func testACircleSelectedByEitherHandleFollowsTheCircle() {
        let circles = [FenceCircle(index: 1, inclusion: true, centre: TrackPoint(41.0, 44.0), radius: 100.0)]
        XCTAssertTrue(selectionSurvives(.Circle(index: 1), [], [], circles, [], []))
        XCTAssertTrue(selectionSurvives(.CircleCentre(index: 1), [], [], circles, [], []))
        XCTAssertFalse(selectionSurvives(.Circle(index: 1), [], [], [], [], []))
    }

    func testAVertexOutsideTheShapeItNamesDoesNotSurvive() {
        let polygons = [FencePolygon(index: 0, inclusion: true, vertices: [TrackPoint(41.0, 44.0), TrackPoint(41.1, 44.1)])]
        XCTAssertTrue(selectionSurvives(.FenceVertex(polygon: 0, vertex: 1), [], polygons, [], [], []))
        XCTAssertFalse(selectionSurvives(.FenceVertex(polygon: 0, vertex: 5), [], polygons, [], [], []))
    }

    func testARallyPointThatHasGoneDoesNotSurvive() {
        let rally = [RallyPoint(index: 3, latitude: 41.0, longitude: 44.0)]
        XCTAssertTrue(selectionSurvives(.Rally(index: 3), [], [], [], rally, []))
        XCTAssertFalse(selectionSurvives(.Rally(index: 4), [], [], [], rally, []))
    }
}

final class SelectedSurveyTests: XCTestCase {
    private func survey(_ index: Int) -> Survey {
        Survey(index: index, area: [TrackPoint(41.0, 44.0)], transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon")
    }

    private var surveys: [Survey] { [survey(1), survey(4)] }

    func testTheSelectedSurveyIsTheOneActedOnNotTheFirst() {
        XCTAssertEqual(4, selectedSurvey(.SurveyVertex(item: 4, vertex: 0), surveys)?.index)
        XCTAssertEqual(1, selectedSurvey(.SurveyVertex(item: 1, vertex: 0), surveys)?.index)
    }

    func testNothingSelectedMeansNoSurveyToActOn() {
        XCTAssertNil(selectedSurvey(nil, surveys))
        XCTAssertNil(selectedSurvey(.Waypoint(index: 2), surveys))
    }

    func testASurveyPickedFromTheListIsTheSameSurveyAsOnePickedByItsCorner() {
        XCTAssertEqual(4, selectedSurvey(.Waypoint(index: 4), surveys)?.index)
        XCTAssertEqual(selectedSurvey(.SurveyVertex(item: 1, vertex: 0), surveys), selectedSurvey(.Waypoint(index: 1), surveys))
    }

    func testOneResolverServingBothKindsDoesNotHandALandingHitBackAsASurvey() {
        XCTAssertNil(selectedSurvey(.LandingPlace(index: 7, place: 0), surveys))
        XCTAssertEqual(4, selectedSurvey(.LandingPlace(index: 4, place: 0), surveys)?.index)
    }

    func testASelectionNamingASurveyThatIsGoneYieldsNothing() {
        XCTAssertNil(selectedSurvey(.SurveyVertex(item: 9, vertex: 0), surveys))
    }
}

final class CornerRemovalTests: XCTestCase {
    private func polygon(_ canRemove: Bool?) -> FencePolygon {
        FencePolygon(
            index: 0,
            inclusion: true,
            vertices: (0..<4).map { TrackPoint(Double($0), Double($0)) },
            editable: canRemove.map { EditableShape(path: "plan.geoFenceController.polygons.0", midpoints: [], splitInvokable: "splitPolygonSegment", canRemoveVertex: $0) }
        )
    }

    func testTheCoreDecidesWhetherACornerCanGoCountingVerticesHereWouldBeASecondOpinion() {
        XCTAssertFalse(cornerRemovable(polygon(false)))
        XCTAssertTrue(cornerRemovable(polygon(true)))
    }

    func testAPolygonThatIsNotThereOffersNothing() {
        XCTAssertFalse(cornerRemovable(nil))
    }

    func testAShapeTheCoreDidNotAnswerForHidesTheRemovalRatherThanGuessing() {
        XCTAssertFalse(cornerRemovable(polygon(nil)))
    }
}
