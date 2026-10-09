import XCTest
@testable import Aircast

final class PlanStripTests: XCTestCase {
    private func item(_ index: Int, _ altitude: Double = .nan) -> MissionItem {
        MissionItem(index: index, sequence: index, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: true, altitude: altitude, altitudeText: altitude.isNaN ? "" : "\(Int(altitude)) m")
    }

    private let summary = JSON.parse(#"{"rows":[{"label":"Distance","value":"1.2 km"},{"label":"Time","value":"00:04:30"},{"label":"Furthest from launch","value":"640 m"}]}"#)

    func testTheStripReadsItemsTheCoresDistanceAndTimeAndTheHighestAltitude() {
        let items = [item(0, 488.0), item(1, 30.0), item(2, 80.0), item(3, 50.0)]
        XCTAssertEqual(
            [PlanStat(label: "Items", value: "3"), PlanStat(label: "Distance", value: "1.2 km"), PlanStat(label: "Time", value: "04:30"), PlanStat(label: "Max alt", value: "80 m")],
            planStats(3, items, summary),
            "the planned home sits at ground height above sea level, so counting it would report the launch site's elevation as the plan's ceiling"
        )
    }

    func testAFlightUnderAnHourDropsTheEmptyHoursSoTheTimeFitsTheStrip() {
        let long = JSON.parse(#"{"rows":[{"label":"Time","value":"01:04:30"}]}"#)
        XCTAssertEqual(PlanStat(label: "Time", value: "01:04:30"), planStats(0, [], long)[1])
    }

    func testAStatTheCoreHasNotWorkedOutIsLeftOffRatherThanShownAsZero() {
        XCTAssertEqual([PlanStat(label: "Items", value: "1")], planStats(1, [item(1)], nil))
        XCTAssertNil(highestAltitude([item(0, 488.0)]))
    }

    func testASelectionThatIsNotAMissionItemIsNamedForWhatItIs() {
        let items = [MissionItem(index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: "Survey", selected: true)]
        XCTAssertEqual(
            ["Fence corner", "Circular fence", "Rally point 3", "Breach return point", "Survey corner"],
            [MapHit.FenceVertex(polygon: 0, vertex: 1), .CircleRadius(index: 0), .Rally(index: 2), .BreachReturn, .SurveyVertex(item: 2, vertex: 0)].map { selectionTitle($0, items) }
        )
    }

    func testAMapTapClosesAnOpenEditorFirstAndAddsTheNextPointOnceItIsFolded() {
        XCTAssertEqual(
            [true, false, false, true],
            [
                tapCloses(.Waypoint(index: 2), true),
                tapCloses(.Waypoint(index: 2), false),
                tapCloses(nil, true),
                tapCloses(.Rally(index: 0), false),
            ]
        )
    }

    func testATerrainConflictIsCountedInLegsWhenTheCoreNamesThem() {
        XCTAssertEqual(
            ["1 leg hits the terrain", "3 legs hit the terrain", "2 items hit the terrain", nil],
            [terrainWarning(1, 1), terrainWarning(3, 2), terrainWarning(0, 2), terrainWarning(0, 0)]
        )
    }

    func testAdvancedNamesTheMissionItemsBehindAWaypointWithActions() {
        let folded = MissionItem(index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: true, altitude: 50.0, foldedCommands: 1)
        XCTAssertEqual("Mission items 3\u{2013}4", advancedDetail(folded))
        XCTAssertNil(advancedDetail(MissionItem(index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: true, altitude: 50.0)))
    }

    func testATakeoffOnThePlannedHomeHidesHomeSoTheMapAndTheStripBothReadT() {
        let home = MissionItem(index: 0, sequence: 0, latitude: 41.0, longitude: 44.0, command: "Home", selected: false, altitude: .nan)
        let takeoff = MissionItem(index: 1, sequence: 1, latitude: 41.0, longitude: 44.0, command: "Takeoff", selected: false, altitude: 50.0, abbreviation: "Takeoff")
        var away = takeoff
        away.latitude = 41.001
        let waypoint = MissionItem(index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0)
        XCTAssertEqual([true, false], [homeCovered([home, takeoff]), homeCovered([home, away])])
        XCTAssertEqual(["H", "T", "2"], [itemSeal(home), itemSeal(takeoff), itemSeal(waypoint)])
        XCTAssertEqual(["Takeoff", "Waypoint 2"], [itemTitle(takeoff), itemTitle(waypoint)])
    }

    func testATemplatesPatternIsTheNewestOneARailPatternTheOneJustInserted() {
        let surveys = [2, 5].map { Survey(index: $0, area: [], transects: [], cameraShots: 0, kind: KIND_SURVEY, shape: "", property: "surveyAreaPolygon") }
        XCTAssertEqual([5, 2, nil], [placedPattern(surveys, NEWEST_PATTERN)?.index, placedPattern(surveys, 2)?.index, placedPattern(surveys, 7)?.index])
    }

    func testWithNothingSelectedTheCoreWeighsNewItemsAtTheEndWhereTheyAreAdded() {
        let items = [0, 1, 2].map { MissionItem(index: $0, sequence: $0, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0) }
            + [MissionItem(index: 3, sequence: 5, latitude: 41.0, longitude: 44.0, command: "Survey", selected: false, altitude: 50.0)]
        XCTAssertEqual(5, appendSequence(items))
        XCTAssertEqual(0, appendSequence([]))
    }

    func testARouteThatBeginsMoreThanAKilometreFromTheAircraftIsFlaggedWithTheDistance() {
        let home = MissionItem(index: 0, sequence: 0, latitude: 37.0, longitude: -122.0, command: "Home", selected: false, altitude: .nan)
        let takeoff = MissionItem(index: 1, sequence: 1, latitude: 39.237, longitude: -123.149, command: "Takeoff", selected: false, altitude: 50.0)
        let zurich = TrackPoint(47.397, 8.545)
        XCTAssertEqual(TrackPoint(39.237, -123.149), routeStart([home, takeoff]))
        XCTAssertNil(farFromAircraft(TrackPoint(39.2371, -123.1491), [home, takeoff]))
        XCTAssertNil(farFromAircraft(nil, [home, takeoff]))
        XCTAssertEqual("Starts 9,", String(startsFromAircraft(farFromAircraft(zurich, [home, takeoff])!, imperial: false).prefix(9)))
    }

    func testDistancesReadInTheOperatorsUnitsCoarserAsTheyGrow() {
        XCTAssertEqual(["850 m", "1.5 km", "9,412 km"], [850.0, 1500.0, 9_412_000.0].map { distanceWords($0, imperial: false) })
        XCTAssertEqual(["492 ft", "1.9 mi", "5,848 mi"], [150.0, 3000.0, 9_412_000.0].map { distanceWords($0, imperial: true) })
    }
}
