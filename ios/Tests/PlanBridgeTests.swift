import XCTest
@testable import Aircast

final class PlanBridgeTests: XCTestCase {
    private func element(
        latitude: Double = 41.0,
        longitude: Double = 44.0,
        specifies: Bool = true,
        sequence: Int = 1,
        command: String = "Waypoint",
        selected: Bool = false,
        withCoordinate: Bool = true
    ) -> String {
        let coordinate = withCoordinate && specifies ? #","coordinate":{"latitude":\#(latitude),"longitude":\#(longitude)}"# : ""
        return #"{"flownLeg":\#(specifies),"sequence":\#(sequence),"name":"\#(command)","selected":\#(selected)\#(coordinate)}"#
    }

    private func model(_ elements: String...) -> JSON { JSON.parse(#"{"kind":"object","items":["# + elements.joined(separator: ",") + "]}") }

    func testALandingDoesNotEndTheRouteItOnlyBreaksTheLegAfterIt() {
        let plan = JSON.parse(#"{"kind":"object","items":[{"sequence":0,"name":"Mission Start","flownLeg":false,"coordinate":{"latitude":41.0,"longitude":44.0}},{"sequence":1,"name":"Waypoint","flownLeg":true,"coordinate":{"latitude":41.1,"longitude":44.1}},{"sequence":2,"name":"Land","flownLeg":true,"endsRoute":true,"command":21,"coordinate":{"latitude":41.2,"longitude":44.2}},{"sequence":3,"name":"Waypoint","flownLeg":true,"coordinate":{"latitude":41.3,"longitude":44.3}}]}"#)
        let items = missionItems(plan)
        XCTAssertEqual([false, false, false, false], items.map(\.afterRouteEnds), "MissionController keeps going past a landing")
        XCTAssertTrue(items[3].routed, "the waypoint after the land is still routed")
        XCTAssertTrue(items[3].legBroken, "but the leg into it is not drawn")
    }

    func testAPlanThatNeverEndsItsRouteStrandsNothing() {
        XCTAssertEqual([false, false], missionItems(model(element(sequence: 1), element(sequence: 2))).map(\.afterRouteEnds))
    }

    func testItemsCarryTheirSequenceCommandAndPosition() {
        let items = missionItems(model(
            element(latitude: 41.1, longitude: 44.1, sequence: 1, command: "Takeoff"),
            element(latitude: 41.2, longitude: 44.2, sequence: 2, selected: true)
        ))
        XCTAssertEqual(2, items.count)
        XCTAssertEqual("Takeoff", items[0].command)
        XCTAssertEqual(1, items[0].sequence)
        XCTAssertEqual(41.1, items[0].latitude, accuracy: 1e-9)
        XCTAssertTrue(items[1].selected, "the editor selection, which is what the core calls it")
    }

    func testItemsWithoutACoordinateOfTheirOwnAreNotPlaced() {
        let items = missionItems(model(element(specifies: false), element(withCoordinate: false), element(latitude: 41.3, longitude: 44.3)))
        XCTAssertEqual(1, items.count)
        XCTAssertEqual(41.3, items[0].latitude, accuracy: 1e-9)
    }

    func testAnUnusableCoordinateIsNotPlaced() {
        XCTAssertEqual(0, missionItems(model(element(latitude: 0.0, longitude: 0.0))).count)
    }

    func testTheIndexSurvivesSkippedItemsSoWritesAddressTheRightOne() {
        XCTAssertEqual([1], missionItems(model(element(specifies: false), element(latitude: 41.4, longitude: 44.4))).map(\.index))
    }

    func testAnEmptyOrAbsentModelYieldsNothing() {
        XCTAssertEqual(0, missionItems(nil).count)
        XCTAssertEqual(0, missionItems(JSON.parse(#"{"kind":"object","elements":[]}"#)).count)
        XCTAssertEqual(0, missionItems(JSON.parse(#"{"kind":"null"}"#)).count)
    }

    func testAnItemReadsTheAltitudeTheCoreResolvedForIt() {
        let withAltitude = #"{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0},"altitude":75.0}"#
        XCTAssertEqual([75.0], missionItems(model(withAltitude)).map(\.altitude))
    }

    func testAnItemTheCoreGaveNoAltitudeReportsItAsUnknown() {
        let plain = #"{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}}"#
        let nulled = #"{"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0},"altitude":null}"#
        XCTAssertEqual([true], missionItems(model(plain)).map(\.altitude.isNaN))
        XCTAssertEqual([true], missionItems(model(nulled)).map(\.altitude.isNaN))
    }

    func testAnItemReadsWhereItLeavesAsWellAsWhereItStarts() {
        let withExit = #"{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},"exitCoordinate":{"latitude":41.5,"longitude":44.5}}"#
        XCTAssertEqual([TrackPoint(41.5, 44.5)], missionItems(model(withExit)).map(\.exit))
    }

    func testAnExitThatRepeatsTheEntryOrCannotBePlottedIsNotCarried() {
        let same = #"{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},"exitCoordinate":{"latitude":41.0,"longitude":44.0}}"#
        let unusable = #"{"specifiesCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0},"exitCoordinate":{"latitude":0.0,"longitude":0.0}}"#
        XCTAssertEqual([nil], missionItems(model(same)).map(\.exit))
        XCTAssertEqual([nil], missionItems(model(unusable)).map(\.exit))
    }

    func testAnItemSaysWhetherTheWaypointLineGoesThroughIt() {
        let roi = #"{"specifiesCoordinate":true,"isStandaloneCoordinate":true,"coordinate":{"latitude":41.0,"longitude":44.0}}"#
        XCTAssertEqual([false], missionItems(model(roi)).map(\.routed))
        XCTAssertEqual([true], missionItems(model(element())).map(\.routed))
    }
}
