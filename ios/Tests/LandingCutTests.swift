import MapLibre
import XCTest
@testable import Aircast

final class LandingCutTests: XCTestCase {
    private func plan(_ items: String...) -> JSON { JSON.parse(#"{"kind":"object","items":[\#(items.joined(separator: ","))]}"#) }

    private func placedAt(_ longitude: Double) -> String {
        #"{"kind":"waypoint","flownLeg":true,"coordinate":{"latitude":41.0,"longitude":\#(longitude)}}"#
    }

    private let rtl = #"{"kind":"command","command":20,"endsRoute":true}"#
    private let landed = #"{"kind":"land","endsRoute":true}"#
    private let settings = #"{"kind":"settings","flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.0}}"#

    func testAnUndrawableLandingStillCutsTheRoute() {
        let items = missionItems(plan(settings, placedAt(44.1), rtl, placedAt(44.2)))
        XCTAssertEqual(3, items.count)
        XCTAssertTrue(items[1].routed)
        XCTAssertFalse(items[2].routed)
    }

    func testNothingIsAfterTheLandingWhenThereIsNoLanding() {
        XCTAssertTrue(missionItems(plan(settings, placedAt(44.1), placedAt(44.2))).allSatisfy(\.routed))
    }

    func testTheRouteIsCutAtTheItemTheCoreMarkedAsEndingIt() {
        XCTAssertEqual(2, routeEndsAfter(plan(settings, placedAt(44.1), rtl, placedAt(44.2))["items"].arrayOrNil))
        XCTAssertEqual(Int.max, routeEndsAfter(nil))
    }

    func testOnlyAReturnEndsTheRouteALandingJustSkipsTheLegAfterIt() {
        XCTAssertEqual(2, routeEndsAfter(plan(settings, placedAt(44.1), rtl, placedAt(44.2))["items"].arrayOrNil))
        XCTAssertEqual(Int.max, routeEndsAfter(plan(settings, placedAt(44.1), landed, placedAt(44.2))["items"].arrayOrNil), "MissionController breaks only at an RTL")
    }

    func testTheLegOutOfALandingIsNotDrawnTheOnesAfterItAre() throws {
        let land = #"{"kind":"land","endsRoute":true,"command":21,"flownLeg":true,"coordinate":{"latitude":41.0,"longitude":44.15}}"#
        let items = missionItems(plan(settings, placedAt(44.1), land, placedAt(44.2), placedAt(44.3)))
        XCTAssertEqual([3], legsAfterLanding(plan(settings, placedAt(44.1), land, placedAt(44.2), placedAt(44.3))["items"].arrayOrNil))
        let path = try XCTUnwrap(missionPath(items, false) as? MLNMultiPolylineFeature)
        let lines = path.polylines.map { line in UnsafeBufferPointer(start: line.coordinates, count: Int(line.pointCount)).map(\.longitude) }
        XCTAssertEqual([[44.1, 44.15], [44.2, 44.3]], lines, "Don't draw segments immediately after a landing item")
        XCTAssertTrue(legArrows(items, false).allSatisfy { !(44.15...44.2).contains($0.at.longitude) })
    }

    func testAnItemTheCoreSaysIsNotAFlownLegIsNeverRouted() {
        let roi = #"{"kind":"roi","flownLeg":false,"coordinate":{"latitude":41.0,"longitude":44.9}}"#
        let items = missionItems(plan(settings, placedAt(44.1), roi, placedAt(44.2)))
        XCTAssertEqual([false], items.filter { $0.command == "" && $0.longitude == 44.9 }.map(\.routed))
        XCTAssertEqual(true, items.first { $0.longitude == 44.1 }?.routed)
    }

    func testTheStrayItemsAfterALandingAreCountedInThePlanShape() {
        XCTAssertEqual(["RTL", "1 after RTL"], planShape(plan(settings, rtl, placedAt(44.2))))
    }

    func testAnItemWithNoUsableCoordinateIsNotPlotted() {
        XCTAssertNil(placed(JSON.parse(#"{"kind":"x"}"#), "coordinate"), "Mission Start carries no coordinate at all")
        XCTAssertNil(placed(JSON.parse(#"{"coordinate":{"latitude":null,"longitude":null}}"#), "coordinate"))
    }
}
