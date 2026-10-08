import XCTest
@testable import Aircast

final class PlanTemplatesTests: XCTestCase {
    func testTemplatesReadFromThePlanViewInTheOrderQgcOffersThem() {
        let read = planTemplates(JSON.parse(#"{"templates":{"show":true,"enabled":false,"prompt":"Click in map to set position","names":["Survey","Corridor Scan","Structure Scan","No Template"],"homeSet":false,"blank":"No Template"}}"#))
        XCTAssertEqual(PlanTemplatesState(show: true, enabled: false, prompt: "Click in map to set position", names: ["Survey", "Corridor Scan", "Structure Scan", "No Template"], homeSet: false, blank: "No Template"), read)
        XCTAssertNil(planTemplates(JSON.parse("{}")))
    }

    func testThePromptSpeaksOfTapsOnATouchScreen() {
        XCTAssertEqual("Tap in map to set position", touchWording("Click in map to set position"))
        XCTAssertEqual("Drag to move home position. Tap to set new position.", touchWording("Drag to move home position. Click to set new position."))
    }

    func testTheBlankMissionComesFirstAndThePromptNamesOneFirstStepUntilHomeIsSet() {
        var state = PlanTemplatesState(show: true, enabled: true, prompt: "Click in map to set position", names: ["Survey", "Corridor Scan", "No Template"], homeSet: false, blank: "No Template")
        let choices = templateChoices(state)
        XCTAssertEqual(["No Template", "Survey", "Corridor Scan"], choices.map(\.0))
        XCTAssertEqual(["Blank mission", "Survey", "Corridor scan"], choices.map(\.1))
        XCTAssertEqual("Tap the map to set home, or start from a template", templatePrompt(state))
        state.homeSet = true
        state.prompt = "Drag to move home position. Click to set new position."
        XCTAssertEqual("Drag to move home position. Tap to set new position.", templatePrompt(state))
    }
}
