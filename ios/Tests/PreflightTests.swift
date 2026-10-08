import XCTest
@testable import Aircast

final class PreflightTests: XCTestCase {
    private let served = #"""
        {"airframe":"Quadrotor · ArduPilot","total":4,"blocked":["Props on"],"groups":[
          {"name":"Before you fly","checks":[
            {"name":"Props on","prompt":"Propellers fitted and tight","verdict":"failing",
             "reason":"Vehicle reports it is not ready to arm.","blocked":true},
            {"name":"Area clear","prompt":"Nobody within the rotor arc","verdict":"manual",
             "reason":"","blocked":false}]},
          {"name":"Vehicle","checks":[
            {"name":"Battery","prompt":"Pack charged and secured","verdict":"passing",
             "reason":"","blocked":false},
            {"name":"Compass","prompt":"Compass calibrated","verdict":"overridable",
             "reason":"Interference detected.","blocked":false}]}]}
        """#

    private func unblocked() -> Preflight {
        var checks = preflight(JSON.parse(served))!
        checks.blocked = []
        return checks
    }

    func testTheGroupsAndTheirChecksComeThroughWithVerdicts() {
        let checks = preflight(JSON.parse(served))!
        XCTAssertEqual(checks.groups.map(\.name), ["Before you fly", "Vehicle"])
        XCTAssertEqual(checks.groups[0].checks.map(\.verdict), ["failing", "manual"])
        XCTAssertEqual(checks.blocked, ["Props on"])
        XCTAssertEqual(checks.total, 4)
    }

    func testOnlyAManualCheckIsTheOperatorsToTick() {
        let checks = preflight(JSON.parse(served))!
        let manual = checks.groups[0].checks.first { $0.name == "Area clear" }!
        let passing = checks.groups[1].checks.first { $0.name == "Battery" }!
        XCTAssertTrue(checkNeedsTicking(manual))
        XCTAssertFalse(checkNeedsTicking(passing))
    }

    func testAFailingCheckShowsTheVehiclesReasonRatherThanAGenericWord() {
        let checks = preflight(JSON.parse(served))!
        let failing = checks.groups[0].checks.first { $0.name == "Props on" }!
        let overridable = checks.groups[1].checks.first { $0.name == "Compass" }!
        XCTAssertEqual(checkStatusText(failing, false), "Vehicle reports it is not ready to arm.")
        XCTAssertEqual(checkStatusText(overridable, false), "Interference detected.")
        XCTAssertEqual(checkStatusText(checks.groups[1].checks[0], false), "Passing")
    }

    func testTheSummaryLeadsWithWhatWouldStopTheFlight() {
        XCTAssertEqual(preflightSummary(preflight(JSON.parse(served)), []), "1 of 4 will stop the flight · 2 left to check.")
    }

    func testTheCountLeftToCheckIsOutOfTheChecksTheOperatorCanTick() {
        XCTAssertEqual(preflightSummary(unblocked(), []), "2 of 2 left to check.")
    }

    func testTickingEveryManualCheckFinishesTheListTheCoreCannotFinishItself() {
        XCTAssertEqual(preflightSummary(unblocked(), ["Area clear", "Compass"]), "All 4 checks done · 1 warning.")
    }

    func testABlockerDoesNotHideTheOperatorsOwnProgress() {
        XCTAssertEqual(preflightSummary(preflight(JSON.parse(served)), ["Area clear", "Compass"]), "1 of 4 will stop the flight.")
    }

    func testACheckTheOperatorCannotTickIsNotDrawnAsAnEmptyBox() {
        let byName = Dictionary(uniqueKeysWithValues: preflight(JSON.parse(served))!.groups.flatMap(\.checks).map { ($0.name, $0) })
        XCTAssertEqual(checkMark(byName["Area clear"]!), .TICKABLE)
        XCTAssertEqual(checkMark(byName["Battery"]!), .PASSED)
        XCTAssertEqual(checkMark(byName["Props on"]!), .ATTENTION)
        XCTAssertEqual(checkMark(byName["Compass"]!), .TICKABLE)
    }

    func testATickForSomethingThatIsNotAManualCheckCountsForNothing() {
        XCTAssertEqual(preflightSummary(unblocked(), ["Battery", "Props on", "made up"]), "2 of 2 left to check.")
    }

    func testNoVehicleIsASentenceNotAnEmptyList() {
        XCTAssertNil(preflight(nil))
        XCTAssertEqual(preflightSummary(nil, []), "Connect a vehicle to run its preflight checks.")
    }
}
