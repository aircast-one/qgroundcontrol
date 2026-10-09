import XCTest
@testable import Aircast

final class RouteSettingsTests: XCTestCase {
    private let stats = [PlanStat(label: "Items", value: "3"), PlanStat(label: "Distance", value: "93 m"), PlanStat(label: "Time", value: "00:35"), PlanStat(label: "Max alt", value: "50.0 m")]

    func testTheTitlePillReadsTheRouteAtAGlance() {
        XCTAssertEqual("3 items \u{00b7} 93 m \u{00b7} 00:35 \u{00b7} max 50.0 m", planStatsLine(stats))
        XCTAssertEqual("1 item", planStatsLine([PlanStat(label: "Items", value: "1")]))
        XCTAssertEqual("", planStatsLine([PlanStat(label: "Items", value: "0")]))
    }

    func testATransferInFlightTakesTheLineAndAnEmptyPlanFallsBackToTheCoresStatus() {
        XCTAssertEqual("Uploading", headerLine("Uploading", true, stats))
        XCTAssertEqual("Empty plan", headerLine("Empty plan", false, [PlanStat(label: "Items", value: "0")]))
    }

    func testTheRouteAltitudeIsTheDefaultEveryNewWaypointTakes() {
        let plan = JSON.parse(#"{"defaults":{"altitude":{"label":"Default altitude","path":"settings.appSettings.defaultMissionItemAltitude","value":60.0,"units":"m"}}}"#)
        XCTAssertEqual(RouteAltitude(value: 60.0, units: "m", path: "settings.appSettings.defaultMissionItemAltitude"), routeAltitude(plan))
        XCTAssertNil(routeAltitude(JSON.parse("{}")))
    }

    func testTheHoldHasItsOwnRowSoTheEditorBelowDoesNotRepeatIt() throws {
        let hold = try fact("Hold")
        let accept = try fact("Acceptance radius")
        XCTAssertEqual([accept], withoutHeroFields([hold, accept], JSON.parse(#"{"hold":{"value":0,"units":"s","path":"p"}}"#)))
        XCTAssertEqual([hold, accept], withoutHeroFields([hold, accept], JSON.parse(#"{"hold":null}"#)))
    }

    private func fact(_ label: String) throws -> Fact {
        try XCTUnwrap(factFromControl(.object(["label": .string(label), "path": .string(label)])))
    }
}
