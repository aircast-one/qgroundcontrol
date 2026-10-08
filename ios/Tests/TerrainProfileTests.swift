import XCTest
@testable import Aircast

private func spans(_ pairs: [(CGFloat, CGFloat)]) -> [[CGFloat]] { pairs.map { [$0.0, $0.1] } }

final class ProfileShapeTests: XCTestCase {
    private func profile(_ points: ProfilePoint...) -> TerrainProfile { TerrainProfile(points: points) }

    private func at(_ distance: Double, _ planned: Double, _ terrain: Double? = nil) -> ProfilePoint {
        ProfilePoint(distance: distance, terrain: terrain, planned: planned)
    }

    func testTheDistanceIsTheLastPoints() {
        XCTAssertEqual(600.0, profile(at(0.0, 50.0), at(600.0, 50.0)).distance, accuracy: 1e-9)
        XCTAssertEqual(0.0, profile().distance, accuracy: 1e-9)
    }

    func testTheBandCoversTheGroundAsWellAsThePlan() {
        let walked = profile(at(0.0, 150.0, 100.0), at(600.0, 150.0, 900.0))
        XCTAssertEqual(100.0, walked.lowest, accuracy: 1e-9)
        XCTAssertEqual(900.0, walked.highest, accuracy: 1e-9)
    }

    func testWithNoGroundTheBandIsThePlanAlone() {
        let flown = profile(at(0.0, 40.0), at(600.0, 90.0))
        XCTAssertEqual(40.0, flown.lowest, accuracy: 1e-9)
        XCTAssertEqual(90.0, flown.highest, accuracy: 1e-9)
    }

    func testOneGroundReadingIsNotATerrainLine() {
        XCTAssertFalse(profile(at(0.0, 50.0, 100.0), at(600.0, 50.0)).hasTerrain)
        XCTAssertTrue(profile(at(0.0, 50.0, 100.0), at(600.0, 50.0, 120.0)).hasTerrain)
    }

    func testCoverageIsTheShareOfTheRouteWithGroundAtBothEnds() {
        XCTAssertEqual(0.5, profile(at(0.0, 50.0, 100.0), at(500.0, 50.0, 100.0), at(1000.0, 50.0)).terrainCoverage, accuracy: 1e-9)
        XCTAssertEqual(0.0, profile().terrainCoverage, accuracy: 1e-9)
    }

    func testAPlanFlownAtOneAltitudeIsFlatButStillDrawable() {
        let level = profile(at(0.0, 50.0), at(600.0, 50.0))
        XCTAssertTrue(level.flat)
        XCTAssertTrue(level.drawable)
        XCTAssertEqual(1.0, level.span, accuracy: 1e-9)
    }

    func testASinglePointIsNotEnoughToDraw() {
        XCTAssertFalse(profile(at(0.0, 50.0)).drawable)
        XCTAssertFalse(profile().drawable)
    }
}

final class ProfileLabelTests: XCTestCase {
    private func profile(_ points: ProfilePoint..., lowest: String = "40 m", band: String = "40 m to 90 m", distance: String = "0.50 km") -> TerrainProfile {
        TerrainProfile(points: points, lowestText: lowest, distanceText: distance, bandText: band)
    }

    private func at(_ distance: Double, _ terrain: Double?, _ planned: Double) -> ProfilePoint {
        ProfilePoint(distance: distance, terrain: terrain, planned: planned)
    }

    func testALevelPlanStatesOneAltitudeRatherThanARange() {
        XCTAssertEqual(
            "50 m AMSL \u{00b7} 6.43 km \u{00b7} ground height unknown",
            profileLabel(profile(at(0.0, nil, 50.0), at(6430.0, nil, 50.0), lowest: "50 m", distance: "6.43 km"))
        )
    }

    func testAClimbingPlanStatesTheBandTheCoreSpelled() {
        XCTAssertEqual("40 m to 90 m AMSL \u{00b7} 0.50 km \u{00b7} ground height unknown", profileLabel(profile(at(0.0, nil, 40.0), at(500.0, nil, 90.0))))
    }

    func testTheBandCarriesWhateverUnitTheCoreSpelledItIn() {
        XCTAssertEqual(
            "131 ft to 295 ft AMSL \u{00b7} 1640 ft \u{00b7} ground height unknown",
            profileLabel(profile(at(0.0, nil, 40.0), at(500.0, nil, 90.0), band: "131 ft to 295 ft", distance: "1640 ft"))
        )
    }

    func testTerrainAtPointsThatDoNotJoinUpIsNotOnePerCentOfTheRoute() {
        let measured = profile(at(0.0, 440.0, 440.0), at(0.0, nil, 490.0), at(36.0, 444.0, 490.0), band: "430 m to 500 m", distance: "36 m")
        XCTAssertEqual(0.0, measured.terrainCoverage, accuracy: 0)
        XCTAssertEqual("430 m to 500 m AMSL \u{00b7} 36 m \u{00b7} ground height at points, none along the route", profileLabel(measured))
    }

    func testGroundUnderPartOfTheRouteSaysHowMuch() {
        XCTAssertEqual(
            "40 m to 90 m AMSL \u{00b7} 1.00 km \u{00b7} ground height for 50% of the route",
            profileLabel(profile(at(0.0, 40.0, 40.0), at(500.0, 45.0, 65.0), at(1000.0, nil, 90.0), distance: "1.00 km"))
        )
    }

    func testASliverOfGroundIsNeverReportedAsNoneOfTheRoute() {
        XCTAssertEqual(
            "40 m to 90 m AMSL \u{00b7} 1.00 km \u{00b7} ground height for 1% of the route",
            profileLabel(profile(at(0.0, 40.0, 40.0), at(1.0, 41.0, 41.0), at(1000.0, nil, 90.0), distance: "1.00 km"))
        )
    }

    func testKnownGroundHeightDropsTheCaveat() {
        XCTAssertEqual("40 m to 90 m AMSL \u{00b7} 0.50 km", profileLabel(profile(at(0.0, 40.0, 40.0), at(500.0, 45.0, 90.0))))
    }
}

final class TerrainViewTests: XCTestCase {
    private func view(_ points: String...) -> JSON {
        JSON.parse(#"{"kind":"object","class":"TerrainProfile","points":["# + points.joined(separator: ",") + "]}")
    }

    private func at(_ distance: Double, _ planned: Double, _ terrain: String) -> String {
        #"{"distance":\#(distance),"missionAltitude":\#(planned),"terrainAltitude":\#(terrain)}"#
    }

    func testTheProfileIsReadStraightFromTheCoresPoints() {
        let profile = terrainProfile(view(at(0.0, 50.0, "948.0"), at(200.0, 100.0, "1000.0")))
        XCTAssertEqual([0.0, 200.0], profile.points.map(\.distance))
        XCTAssertEqual([50.0, 100.0], profile.points.map(\.planned))
        XCTAssertEqual([948.0, 1000.0], profile.points.map(\.terrain))
        XCTAssertEqual(200.0, profile.distance, accuracy: 1e-9)
    }

    func testGroundTheCoreCouldNotResolveStaysUnknownRatherThanBecomingZero() {
        let profile = terrainProfile(view(at(0.0, 50.0, "null"), at(200.0, 100.0, "1000.0")))
        XCTAssertEqual([nil, 1000.0], profile.points.map(\.terrain))
        XCTAssertEqual(50.0, profile.lowest, accuracy: 1e-9)
    }

    func testAPointWithNoPlannedAltitudeStaysOnTheDistanceAxisButDrawsNoFlightLine() {
        let profile = terrainProfile(view(#"{"distance":10.0,"terrainAltitude":900.0}"#))
        XCTAssertEqual(1, profile.points.count)
        XCTAssertTrue(profile.points[0].planned.isNaN)
    }

    func testAnAbsentOrEmptyViewDrawsNothing() {
        XCTAssertEqual(0, terrainProfile(nil).points.count)
        XCTAssertFalse(terrainProfile(view()).drawable)
    }
}

final class HeightRangeTests: XCTestCase {
    private func profile(_ low: Double, _ high: Double, lowText: String = "", highText: String = "", bandText: String = "") -> TerrainProfile {
        TerrainProfile(
            points: [ProfilePoint(distance: 0.0, terrain: nil, planned: low), ProfilePoint(distance: 100.0, terrain: nil, planned: high)],
            lowestText: lowText,
            highestText: highText,
            bandText: bandText
        )
    }

    func testTheBandIsSpelledByTheCoreWhichGivesBothEndsOnePrecision() {
        XCTAssertEqual("-33 ft to 197 ft AMSL", heightRange(profile(-10.0, 60.0, lowText: "-32.8 ft", highText: "197 ft", bandText: "-33 ft to 197 ft")))
    }

    func testAFlatRouteNamesOneHeightNotARangeOfOne() {
        XCTAssertEqual("50 m AMSL", heightRange(profile(50.0, 50.0, lowText: "50 m", highText: "50 m", bandText: "50 m to 50 m")))
    }

    func testMarkersAndCollisionsComeFromTheCoreAndATapPicksTheNearestItem() {
        let profile = terrainProfile(JSON.parse(#"{"points":[{"distance":0,"missionAltitude":100,"terrainAltitude":50},{"distance":100,"missionAltitude":100,"terrainAltitude":120,"collision":true},{"distance":200,"missionAltitude":100,"terrainAltitude":130,"collision":true},{"distance":1000,"missionAltitude":100,"terrainAltitude":50}],"markers":[{"sequence":0,"distance":0,"label":"T"},{"sequence":2,"distance":200,"label":"2","complex":{"endDistance":600,"lastSequence":7,"pattern":"Survey"}}]}"#))
        XCTAssertEqual(["T", "2"], profile.markers.map(\.label))
        XCTAssertEqual(7, profile.markers[1].lastSequence)
        XCTAssertEqual(1, collisionSegments(profile, 1000, 100).count)
        XCTAssertEqual([0, 2, 2, nil], [10, 590, 400, 850].map { tappedSequence(profile, 1000, $0) }, "the exit line and the band between belong to the pattern; open ground selects nothing")
    }

    func testTheProfileFollowsShowMissionItemStatusShownByDefaultLikePlanViewSettings() {
        let settings: [JSON?] = [nil, JSON.parse(#"{"value":null}"#), JSON.parse(#"{"value":false}"#), JSON.parse(#"{"value":true}"#)]
        XCTAssertEqual([true, true, false, true], settings.map(missionItemStatusShown))
    }
}

final class TerrainGapTests: XCTestCase {
    private let profile = TerrainProfile(
        points: [
            ProfilePoint(distance: 0.0, terrain: 100.0, planned: 150.0),
            ProfilePoint(distance: 100.0, terrain: 110.0, planned: 150.0),
            ProfilePoint(distance: 200.0, terrain: nil, planned: 150.0),
            ProfilePoint(distance: 300.0, terrain: 120.0, planned: 150.0),
            ProfilePoint(distance: 400.0, terrain: 130.0, planned: 150.0),
        ],
        band: (90.0, 160.0)
    )

    func testMissingGroundBreaksTheTerrainLineAndIsMarkedAlongTheBottomLikeTerrainProfile() {
        XCTAssertEqual([2, 2], terrainRuns(profile, 400, 70).map(\.count))
        XCTAssertEqual([[100, 200], [200, 300]], spans(missingSpans(profile, 400)))
        XCTAssertEqual(90.0, profile.lowest, accuracy: 0, "the chart uses the core's padded band")
        let waiting = terrainProfile(JSON.parse(#"{"minAltitudeMeters":90,"maxAltitudeMeters":160,"points":[{"distance":0,"missionAltitude":150,"terrainAltitude":100},{"distance":100,"missionAltitude":150,"terrainAltitude":110},{"distance":100,"missionAltitude":null},{"distance":300,"missionAltitude":null},{"distance":300,"missionAltitude":150,"terrainAltitude":120},{"distance":400,"missionAltitude":150,"terrainAltitude":130}]}"#))
        XCTAssertEqual([2, 2], plannedRuns(waiting, 400, 70).map(\.count), "a span with no heights yet keeps its points, draws no flight line there")
        XCTAssertEqual([[100, 100], [100, 300], [300, 300]], spans(missingSpans(waiting, 400)))
    }

    func testTheProfileAndItsChipBelongToTheMissionLayerAsPlanViewShowsTerrainStatusOnlyThere() {
        let drawn = TerrainProfile(points: [ProfilePoint(distance: 0.0, terrain: nil, planned: 10.0)])
        XCTAssertTrue(profileShown(.Mission, drawn))
        XCTAssertFalse(profileShown(.Fence, drawn))
        XCTAssertFalse(profileShown(.Mission, TerrainProfile(points: [])))
    }
}
