import XCTest
@testable import Aircast

final class PlanTemplatesTests: XCTestCase {
    func testTemplatesReadFromThePlanViewInTheOrderQgcOffersThem() {
        let read = planTemplates(JSON.parse(#"{"templates":{"show":true,"enabled":false,"prompt":"Click in map to set position","names":["Survey","Corridor Scan","Structure Scan","No Template"],"homeSet":false,"blank":"No Template"}}"#))
        XCTAssertEqual(PlanTemplatesState(enabled: false, names: ["Survey", "Corridor Scan", "Structure Scan", "No Template"], blank: "No Template"), read)
        XCTAssertNil(planTemplates(JSON.parse("{}")))
    }

    func testTheBlankMissionComesFirstAndStandsForANewEmptyPlan() {
        let state = PlanTemplatesState(enabled: true, names: ["Survey", "Corridor Scan", "No Template"], blank: "No Template")
        let choices = templateChoices(state)
        XCTAssertEqual([nil, "Survey", "Corridor Scan"], choices.map(\.0))
        XCTAssertEqual(["Blank mission", "Survey", "Corridor scan"], choices.map(\.1))
    }

    func testWithNoAnswerFromTheCoreABlankPlanIsStillOffered() {
        let choices = templateChoices(nil)
        XCTAssertEqual([nil], choices.map(\.0))
        XCTAssertEqual(["Blank mission"], choices.map(\.1))
    }
}
