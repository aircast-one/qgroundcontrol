import XCTest
@testable import Aircast

private func waypointAt(_ latitude: Double, _ longitude: Double) -> MissionItem {
    MissionItem(index: 0, sequence: 1, latitude: latitude, longitude: longitude, command: "Waypoint", selected: false, altitude: .nan)
}

final class PlanBoundsTests: XCTestCase {
    func testBoundsCoverEveryCornerOfThePlan() throws {
        let bounds = try XCTUnwrap(planBounds(planPoints([waypointAt(41.0, 44.0), waypointAt(41.5, 44.5), waypointAt(41.2, 43.8)])))
        XCTAssertEqual(41.0, bounds.south, accuracy: 1e-9)
        XCTAssertEqual(43.8, bounds.west, accuracy: 1e-9)
        XCTAssertEqual(41.5, bounds.north, accuracy: 1e-9)
        XCTAssertEqual(44.5, bounds.east, accuracy: 1e-9)
    }

    func testACircleContributesItsRingSoAFitDoesNotCutItInHalf() throws {
        let centre = TrackPoint(41.0, 44.0)
        let bounds = try XCTUnwrap(planBounds(planPoints([], [], [FenceCircle(index: 0, inclusion: true, centre: centre, radius: 500.0)])))
        XCTAssertTrue(bounds.north > centre.latitude)
        XCTAssertTrue(bounds.south < centre.latitude)
        XCTAssertTrue(bounds.east > centre.longitude)
        XCTAssertTrue(bounds.west < centre.longitude)
    }

    func testFencesRallyPointsAndSurveysAllCount() throws {
        let points = planPoints(
            [],
            [FencePolygon(index: 0, inclusion: true, vertices: [TrackPoint(40.0, 43.0)])],
            [],
            [RallyPoint(index: 0, latitude: 42.0, longitude: 45.0)],
            [Survey(index: 0, area: [TrackPoint(41.0, 44.0)], transects: [TrackPoint(41.9, 44.9)], cameraShots: 0, kind: KIND_SURVEY, shape: SHAPE_AREA, property: "surveyAreaPolygon")]
        )
        let bounds = try XCTUnwrap(planBounds(points))
        XCTAssertEqual(40.0, bounds.south, accuracy: 1e-9)
        XCTAssertEqual(43.0, bounds.west, accuracy: 1e-9)
        XCTAssertEqual(42.0, bounds.north, accuracy: 1e-9)
        XCTAssertEqual(45.0, bounds.east, accuracy: 1e-9)
    }

    func testAnEmptyOrUnplottablePlanHasNoBoundsToFit() {
        XCTAssertNil(planBounds([]))
        XCTAssertNil(planBounds([TrackPoint(0.0, 0.0)]))
    }

    func testASinglePointStillYieldsBoundsCentredOnItself() throws {
        let bounds = try XCTUnwrap(planBounds([TrackPoint(41.0, 44.0)]))
        XCTAssertEqual(TrackPoint(41.0, 44.0), bounds.centre)
        XCTAssertEqual(0.0, bounds.spanDegrees, accuracy: 1e-9)
    }
}

final class FitPointsTests: XCTestCase {
    func testAPlanIsFramedOnItsOwnTerms() {
        let plan = planPoints([waypointAt(41.0, 44.0), waypointAt(41.5, 44.5)])
        XCTAssertEqual(plan, fitPoints(plan, -35.36, 149.16))
    }

    func testAnEmptyPlanFallsBackToTheAircraft() {
        XCTAssertEqual([TrackPoint(-35.36, 149.16)], fitPoints([], -35.36, 149.16))
    }

    func testWithNoPlanAndNoPositionThereIsNothingToFrame() {
        XCTAssertTrue(fitPoints([], .nan, .nan).isEmpty)
        XCTAssertTrue(fitPoints([], 0.0, 0.0).isEmpty)
    }
}

final class LongitudeWraparoundTests: XCTestCase {
    private func at(_ longitudes: Double...) throws -> PlanBounds {
        try XCTUnwrap(planBounds(longitudes.map { TrackPoint(10.0, $0) }))
    }

    func testTwoPointsEitherSideOfTheAntimeridianDoNotFrameThePlanet() throws {
        let bounds = try at(179.9, -179.9)
        XCTAssertEqual(0.2, bounds.longitudeSpan, accuracy: 1e-9)
        XCTAssertEqual(180.0, bounds.centre.longitude, accuracy: 1e-9)
    }

    func testAnOrdinaryRegionIsUnchangedByTheArcTreatment() throws {
        let bounds = try at(44.7, 44.9, 44.8)
        XCTAssertEqual(44.7, bounds.west, accuracy: 1e-9)
        XCTAssertEqual(44.9, bounds.east, accuracy: 1e-9)
        XCTAssertEqual(0.2, bounds.longitudeSpan, accuracy: 1e-9)
        XCTAssertEqual(44.8, bounds.centre.longitude, accuracy: 1e-9)
    }

    func testTheWidestEmptyGapIsTheOneLeftOutOfTheArc() throws {
        let bounds = try at(-10.0, 10.0, 170.0)
        XCTAssertEqual(-10.0, bounds.west, accuracy: 1e-9)
        XCTAssertEqual(170.0, bounds.east, accuracy: 1e-9)
        XCTAssertEqual(180.0, bounds.longitudeSpan, accuracy: 1e-9)
        XCTAssertEqual(80.0, bounds.centre.longitude, accuracy: 1e-9)
    }

    func testASinglePointSpansNothingAndCentresOnItself() throws {
        let bounds = try at(-179.95)
        XCTAssertEqual(0.0, bounds.longitudeSpan, accuracy: 1e-9)
        XCTAssertEqual(-179.95, bounds.centre.longitude, accuracy: 1e-9)
    }

    func testTheSpanNeverExceedsThePlanet() throws {
        let bounds = try at(-179.0, -90.0, 0.0, 90.0, 179.0)
        XCTAssertTrue(bounds.longitudeSpan <= 360.0)
        XCTAssertEqual(270.0, bounds.longitudeSpan, accuracy: 1e-9)
    }

    func testNormalisingKeepsALongitudeInRange() {
        XCTAssertEqual(-179.0, normaliseLongitude(181.0), accuracy: 1e-9)
        XCTAssertEqual(179.0, normaliseLongitude(-181.0), accuracy: 1e-9)
        XCTAssertEqual(180.0, normaliseLongitude(180.0), accuracy: 1e-9)
        XCTAssertEqual(0.0, normaliseLongitude(720.0), accuracy: 1e-9)
    }
}

final class TakeoffMissingTests: XCTestCase {
    private func item(_ kind: String, _ name: String, _ lat: Double = 41.0, _ lon: Double = 44.0) -> MissionItem {
        MissionItem(index: 0, sequence: 1, latitude: lat, longitude: lon, command: name, selected: false, altitude: 50.0, kind: kind)
    }

    func testWaypointsWithNoTakeoffNeedOneInsertedFirst() {
        XCTAssertTrue(takeoffMissing([item("waypoint", "Waypoint"), item("waypoint", "Waypoint")]))
    }

    func testAPlanThatAlreadyHasATakeoffDoesNotGetAnother() {
        XCTAssertFalse(takeoffMissing([item("takeoff", "Takeoff"), item("waypoint", "Waypoint")]))
    }

    func testAVtolTakeoffCountsAsATakeoff() {
        XCTAssertFalse(takeoffMissing([item("takeoff", "VTOL Takeoff"), item("waypoint", "Waypoint")]))
    }

    func testATakeoffNamedInAnotherLanguageStillCounts() {
        XCTAssertFalse(takeoffMissing([item("takeoff", "Starten"), item("waypoint", "Wegpunkt")]))
    }

    func testAnEmptyPlanNeedsNothingInsertedBeforeAnything() {
        XCTAssertFalse(takeoffMissing([]))
    }

    func testItemsWithNoCoordinateDoNotByThemselvesRequireATakeoff() {
        XCTAssertFalse(takeoffMissing([item("command", "Return To Launch", 0.0, 0.0)]))
    }
}

final class FitsPlanOnEntryTests: XCTestCase {
    func testAPlanThatWasAlreadyThereIsWhatTheTabOpensOn() {
        XCTAssertTrue(fitsPlanOnEntry(true, true, true))
    }

    func testAnEmptyPlanLeavesTheCameraOnTheVehicle() {
        XCTAssertFalse(fitsPlanOnEntry(true, true, false))
    }

    func testTheFirstItemAddedLaterDoesNotYankTheCamera() {
        XCTAssertFalse(fitsPlanOnEntry(false, true, true))
    }

    func testAReadThatNeverReachedThePlanDoesNotSpendTheOneChanceToFit() {
        XCTAssertFalse(fitsPlanOnEntry(true, false, false))
        XCTAssertTrue(stillFirstRead(true, false))
    }

    func testAReadThatReachedThePlanSettlesItWhateverThePlanHeld() {
        XCTAssertFalse(stillFirstRead(true, true))
        XCTAssertFalse(stillFirstRead(false, false))
    }

    func testAPlanIsDrawnIfAnythingAtAllIsOnTheMap() {
        XCTAssertFalse(planIsDrawn([], [], [], [], []))
        XCTAssertTrue(planIsDrawn([], [], [], [], [RallyPoint(index: 0, latitude: 41.0, longitude: 44.0)]))
    }

    func testThePlanMapCentresOnTheVehicleOnceOnEntryAndThenStaysWhereTheOperatorPutIt() {
        XCTAssertTrue(centersOnVehicleAtEntry(false, true, 0))
        XCTAssertFalse(centersOnVehicleAtEntry(true, true, 0), "PlanView never follows the vehicle; the Center menu moves the map")
        XCTAssertFalse(centersOnVehicleAtEntry(false, true, 1), "a mission fitted on entry wins")
        XCTAssertFalse(centersOnVehicleAtEntry(false, false, 0))
    }
}
