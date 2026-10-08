import XCTest
@testable import Aircast

final class LandingPatternTests: XCTestCase {
    private func view(_ body: String) -> JSON { JSON.parse(body) }

    private let whole = """
        {"landing":{"latitude":41.70,"longitude":44.82,"altitude":0},
         "slopeStart":{"latitude":41.71,"longitude":44.83,"altitude":40},
         "finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
         "loiterRadiusMetres":75.0,"loiterClockwise":true}
        """

    func testAPatternIsDrawnFromApproachThroughSlopeStartToTheLanding() throws {
        let pattern = try XCTUnwrap(landingPattern(4, view(whole)))
        XCTAssertEqual(approachPath(pattern), [pattern.finalApproach, pattern.slopeStart, pattern.landing].compactMap { $0 })
        XCTAssertEqual(landingPathFeatures([pattern]).shapes.count, 1)
    }

    func testAnUnsetCornerIsAbsentSoThePatternIsNeverPlottedAtNullIsland() throws {
        let partial = try XCTUnwrap(landingPattern(4, view("""
            {"landing":null,"slopeStart":null,
             "finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
             "loiterRadiusMetres":75.0}
            """)))
        XCTAssertNil(partial.landing)
        XCTAssertEqual(approachPath(partial), [partial.finalApproach].compactMap { $0 })
    }

    func testAPatternWithOnePlaceDrawsNoPathHavingNothingToJoin() throws {
        let one = try XCTUnwrap(landingPattern(4, view(#"{"finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60}}"#)))
        XCTAssertEqual(landingPathFeatures([one]).shapes.count, 0)
    }

    func testTheLoiterCircleIsDrawnAroundTheApproachAndOnlyWhenItHasARadius() throws {
        let pattern = try XCTUnwrap(landingPattern(4, view(whole)))
        XCTAssertEqual(landingLoiterFeatures([pattern]).shapes.count, 0, "FWLandingPatternMapVisual shows the circle only with loiter-to-altitude")
        XCTAssertEqual(landingLoiterFeatures([withChanges(pattern) { $0.loiterToAltitude = true }]).shapes.count, 1)
        let noRadius = try XCTUnwrap(landingPattern(4, view("""
            {"finalApproach":{"latitude":41.72,"longitude":44.84,"altitude":60},
             "landing":{"latitude":41.70,"longitude":44.82,"altitude":0}}
            """)))
        XCTAssertEqual(landingLoiterFeatures([noRadius]).shapes.count, 0)
    }

    func testAnItemWithNoPatternIsNotOneHoweverTheCoreSaysSo() {
        XCTAssertNil(landingPattern(4, nil))
        XCTAssertNil(landingPattern(4, view(#"{"kind":"null"}"#)))
        XCTAssertNil(landingPattern(4, view(#"{"reason":"only a fixed wing or a VTOL gets one"}"#)))
        XCTAssertNil(landingPattern(4, view(#"{"loiterRadiusMetres":75.0}"#)))
    }
}

final class LandingHandleTests: XCTestCase {
    private let pattern = LandingPattern(
        index: 4,
        landing: TrackPoint(latitude: 41.70, longitude: 44.82),
        slopeStart: TrackPoint(latitude: 41.71, longitude: 44.83),
        finalApproach: TrackPoint(latitude: 41.72, longitude: 44.84),
        loiterRadiusMetres: 75.0,
        loiterClockwise: true
    )

    func testOnlyTheTwoPlacesQgcLetsYouSetGetAHandle() {
        let handles = vertexHandleFeatures([], [], circles: [], landings: [pattern]).shapes
        XCTAssertEqual(handles.count, 2)
        XCTAssertEqual(handles.map { $0.getNumberProperty(VERTEX_INDEX_PROPERTY).map { Int($0) } }, [LANDING_PLACE_APPROACH, LANDING_PLACE_TOUCHDOWN])
        XCTAssertEqual(handles.map { $0.getStringProperty(HANDLE_KIND_PROPERTY) }, [HANDLE_KIND_LANDING, HANDLE_KIND_LANDING])
    }

    func testAnUnplacedPatternOffersNothingToDrag() {
        let empty = withChanges(pattern) {
            $0.landing = nil
            $0.slopeStart = nil
            $0.finalApproach = nil
        }
        XCTAssertEqual(vertexHandleFeatures([], [], circles: [], landings: [empty]).shapes.count, 0)
    }

    func testASelectionOnALandingPlaceSurvivesWhileThePatternDoes() {
        XCTAssertTrue(selectionSurvives(.LandingPlace(index: 4, place: LANDING_PLACE_APPROACH), [], [], [], [], [], landings: [pattern]))
        XCTAssertFalse(selectionSurvives(.LandingPlace(index: 9, place: LANDING_PLACE_APPROACH), [], [], [], [], [], landings: [pattern]))
    }

    func testEachPlaceSaysWhichOneMoved() {
        XCTAssertEqual(movedText(.LandingPlace(index: 4, place: LANDING_PLACE_APPROACH), []), "Moved the final approach")
        XCTAssertEqual(movedText(.LandingPlace(index: 4, place: LANDING_PLACE_TOUCHDOWN), []), "Moved the touchdown")
    }
}

final class IsLandingPatternTests: XCTestCase {
    func testARefusalIsNotAPatternAndNeitherIsAnAbsentView() {
        XCTAssertFalse(isLandingPattern(nil))
        XCTAssertFalse(isLandingPattern(JSON.parse(#"{"kind":"null"}"#)))
        XCTAssertFalse(isLandingPattern(JSON.parse(#"{"reason":"only a fixed wing or a VTOL gets one"}"#)))
    }

    func testAPatternWithNoPlacesIsStillAPatternWhichIsTheWholeDistinction() {
        let unplaced = JSON.parse(#"{"landing":null,"slopeStart":null,"finalApproach":null}"#)
        XCTAssertTrue(isLandingPattern(unplaced))
        XCTAssertNil(landingPattern(4, unplaced))
    }

    func testALoiterItemsRingIsItsRadiusWhicheverWayItTurns() {
        let loiter = MissionItem(index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: "", selected: false, loiterRadius: -80.0)
        let waypoint = MissionItem(index: 1, sequence: 1, latitude: 47.1, longitude: 8.0, command: "", selected: false)
        let rings = loiterRings([], [waypoint, loiter])
        XCTAssertEqual(rings.map(\.0), [TrackPoint(latitude: 47.0, longitude: 8.0)])
        XCTAssertEqual(rings.map(\.1), [80.0])
    }

    func testTheLandingAreaIsA15By100MetreBoxOnTheTouchdownAndTheGlideSlopeRunsToTheApproach() throws {
        let landing = TrackPoint(latitude: 47.0, longitude: 8.0)
        let slope = pointAt(landing, 400.0, 90.0)
        let approach = pointAt(landing, 800.0, 90.0)
        let straight = LandingPattern(index: 3, landing: landing, slopeStart: slope, finalApproach: approach, loiterRadiusMetres: nil, loiterClockwise: true)
        let area = try XCTUnwrap(landingArea(straight))
        XCTAssertEqual(area.count, 4)
        XCTAssertEqual(metresBetween(landing, area[0]), hypot(7.5, 50.0), accuracy: 0.5, "corners are the half-diagonal from touchdown")
        XCTAssertEqual(try XCTUnwrap(glideSlope(straight)).last, approach, "without loiter-to-altitude the slope reaches the final approach")
        XCTAssertEqual(try XCTUnwrap(glideSlope(withChanges(straight) { $0.loiterRadiusMetres = 75.0 })).last, approach, "a radius alone is not loiter-to-altitude")
        XCTAssertEqual(
            try XCTUnwrap(glideSlope(withChanges(straight) { $0.loiterRadiusMetres = 75.0; $0.loiterToAltitude = true })).last,
            slope,
            "with it, the slope start"
        )
        let labels = landingLabels(withChanges(straight) { $0.heights = GlideSlopeHeights(transition: "5 m*", midSlope: "52 m*", approach: "100.0 m") }).map(\.text)
        XCTAssertEqual(labels, ["Landing Area", "Glide Slope", "5 m*", "52 m*", "100.0 m"], "FWLandingPatternMapVisual's two names and three heights")
        XCTAssertEqual(landingAreaFeatures([straight]).shapes.count, 2, "FWLandingPatternMapVisual draws both whether or not the item is current")
        XCTAssertEqual(landingLabelFeatures([straight], 2).shapes.count, 0, "only the current item's labels show")
    }

    func testAStructureScanNamesItsEntryAndExit() {
        let scan = MissionItem(index: 2, sequence: 2, latitude: 47.0, longitude: 8.0, command: "", selected: false, exit: TrackPoint(latitude: 47.1, longitude: 8.0), kind: KIND_STRUCTURE)
        XCTAssertEqual(structureScanLabels([scan]).map(\.text), ["Entry", "Exit"])
        XCTAssertEqual(
            structureScanLabels([withChanges(scan) { $0.exit = nil }]).map(\.at.latitude),
            [scan.latitude, scan.latitude],
            "exitCoordinateSameAsEntry: the core sends no exit, both labels sit on the entry"
        )
    }

    func testACollidingGlideSlopeAndLoiterCircleTurnRedLikeTheirQgcVisuals() {
        let slope = LandingPattern(
            index: 3,
            landing: TrackPoint(latitude: 47.0, longitude: 8.0),
            slopeStart: TrackPoint(latitude: 47.01, longitude: 8.0),
            finalApproach: TrackPoint(latitude: 47.02, longitude: 8.0),
            loiterRadiusMetres: 80.0,
            loiterClockwise: true,
            collides: true
        )
        let shapes = Dictionary(uniqueKeysWithValues: landingAreaFeatures([slope]).shapes.map {
            ($0.getStringProperty(LANDING_SHAPE_KIND) ?? "", $0.getBooleanProperty(TERRAIN_COLLISION))
        })
        XCTAssertEqual(shapes, [LANDING_AREA_KIND: false, GLIDE_SLOPE_KIND: true], "only the glide slope takes the collision colour")
        let loiter = MissionItem(index: 4, sequence: 4, latitude: 47.0, longitude: 8.0, command: "Loiter", selected: false, altitude: 50.0, loiterRadius: 80.0, terrainCollision: true)
        let drawn = landingLoiterFeatures([], items: [loiter]).shapes
        XCTAssertEqual(drawn.count, 1)
        XCTAssertEqual(drawn.first?.getBooleanProperty(TERRAIN_COLLISION), true)
    }

    func testAVtolLandingPatternDrawsNoLandingAreaOrGlideSlopeAsVtolLandingPatternMapVisual() {
        let vtol = LandingPattern(
            index: 3,
            landing: TrackPoint(latitude: 47.0, longitude: 8.0),
            slopeStart: TrackPoint(latitude: 47.01, longitude: 8.0),
            finalApproach: TrackPoint(latitude: 47.02, longitude: 8.0),
            loiterRadiusMetres: 80.0,
            loiterClockwise: true,
            heights: GlideSlopeHeights(transition: "1", midSlope: "2", approach: "3"),
            glideSlopeShown: false
        )
        XCTAssertEqual(landingAreaFeatures([vtol]).shapes.count, 0)
        XCTAssertEqual(landingLabels(vtol), [])
    }

    func testALandingPatternIsMarkedAtItsApproachAndTouchdownLikeQgcsVisuals() throws {
        let item = MissionItem(index: 3, sequence: 7, latitude: 47.0, longitude: 8.0, command: "Landing Pattern", selected: false, altitude: 30.0, kind: KIND_LAND, foldedCommands: 4)
        let pattern = LandingPattern(
            index: 3,
            landing: TrackPoint(latitude: 47.0, longitude: 8.0),
            slopeStart: TrackPoint(latitude: 47.01, longitude: 8.0),
            finalApproach: TrackPoint(latitude: 47.02, longitude: 8.0),
            loiterRadiusMetres: 80.0,
            loiterClockwise: true
        )
        let marks = missionFeatures([item], landings: [pattern]).shapes
        XCTAssertEqual(marks.map { $0.getStringProperty(WAYPOINT_LABEL_PROPERTY) }, ["7", "11"])
        XCTAssertEqual(marks.map { $0.getStringProperty(WAYPOINT_SIDE_LABEL_PROPERTY) }, ["Approach", "Land"])
        let loiter = missionFeatures([item], landings: [withChanges(pattern) { $0.loiterToAltitude = true }]).shapes.first?.getStringProperty(WAYPOINT_SIDE_LABEL_PROPERTY)
        XCTAssertEqual(loiter, "Loiter")
    }
}
