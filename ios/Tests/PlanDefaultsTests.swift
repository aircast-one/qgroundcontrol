import XCTest
@testable import Aircast

final class PlanDefaultsTests: XCTestCase {
    private func control(_ name: String, _ value: Double, _ units: String) -> String {
        #"""
        {"kind":"object","class":"Control","control":"number","name":"\#(name)","label":"\#(name)",
            "path":"settings.appSettings.\#(name)","value":\#(value),"valueString":"\#(value)",
            "units":"\#(units)","enabled":true,"readOnly":false,"options":[],"bits":[]}
        """#
    }

    private func plan(_ defaults: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Plan","defaults":\#(defaults)}"#)
    }

    func testTheThreeTheCoreServesAreTheThreeOffered() {
        let facts = planDefaults(plan(#"""
        {"altitude":\#(control("defaultMissionItemAltitude", 50, "m")),
            "cruise":\#(control("offlineEditingCruiseSpeed", 15, "m/s")),
            "hover":\#(control("offlineEditingHoverSpeed", 5, "m/s")),
            "speedUnits":"m/s"}
        """#))
        XCTAssertEqual(
            ["defaultMissionItemAltitude", "offlineEditingCruiseSpeed", "offlineEditingHoverSpeed"],
            facts.map(\.name),
            "the altitude every new mission item starts at, and the speeds a plan edited with no vehicle is flown at - none of which was on any screen, so an operator adding a waypoint got a height they could not see or change from the Plan tab"
        )
    }

    func testTheOrderIsTheListsNotWhateverOrderTheKeysArriveIn() {
        let facts = planDefaults(plan(#"""
        {"hover":\#(control("hover", 5, "m/s")),
            "cruise":\#(control("cruise", 15, "m/s")),
            "altitude":\#(control("altitude", 50, "m"))}
        """#))
        XCTAssertEqual(
            ["altitude", "cruise", "hover"],
            facts.map(\.name),
            "JSONObject.keys() has no defined order on Android, so height before speeds is what PLAN_DEFAULT_KEYS is for - the filtering is structural and would work without it"
        )
    }

    func testAValueThatIsNotAnObjectCannotBecomeAControl() {
        let facts = planDefaults(plan(#"{"altitude":\#(control("a", 50, "m")),"speedUnits":"ft/s"}"#))
        XCTAssertEqual(1, facts.count)
        XCTAssertEqual("a", facts.first?.name)
    }

    func testACoreTooOldToServeDefaultsSaysSoRatherThanShowingAnEmptyDialog() {
        XCTAssertTrue(planDefaults(JSON.parse(#"{"kind":"object"}"#)).isEmpty)
        XCTAssertTrue(planDefaults(nil).isEmpty)
        XCTAssertTrue(planDefaultsNote(nil, []).contains("does not report"))
    }

    func testWithDefaultsPresentTheNoteIsTheCoresSpeedNoteLikeMissionSettingsEditor() {
        let view = plan(#"{"altitude":\#(control("a", 50, "m")),"speedNote":"Speeds are used to estimate mission time only. They do not change the flight speed."}"#)
        XCTAssertEqual("Speeds are used to estimate mission time only. They do not change the flight speed.", planDefaultsNote(view, planDefaults(view)))
        let noSpeeds = plan(#"{"altitude":\#(control("a", 50, "m")),"speedNote":null}"#)
        XCTAssertEqual("", planDefaultsNote(noSpeeds, planDefaults(noSpeeds)))
    }

    func testTheMissionFlightSpeedIsTheSettingsItemsSpeedSectionWithSpeedSectionsUserRange() {
        let view = plan(#"""
        {"flightSpeed":{"available":true,"specified":true,"value":7.5,"units":"m/s","slider":{"from":0.0,"to":30.0,"decimals":1},
            "path":"plan.missionController.visualItems.0.speedSection.flightSpeed","specifyPath":"plan.missionController.visualItems.0.speedSection.specifyFlightSpeed"}}
        """#)
        let speed = speedSectionOf(view["defaults"]["flightSpeed"])!
        XCTAssertEqual(FactSlider(from: 0, to: 30, decimals: 1, hint: ""), speed.slider)
        XCTAssertEqual(7.5, speed.value)
        XCTAssertNil(speedSectionOf(JSON.parse(#"{"available":false}"#)))
    }

    func testADefaultCarryingTheUserRangeGetsTheSliderMissionDefaultsEditorDrawsUnderItsField() {
        let ranged = control("offlineEditingCruiseSpeed", 15, "m/s").replacingOccurrences(of: #""bits":[]"#, with: #""bits":[],"slider":{"from":1.0,"to":30.0,"decimals":1}"#)
        let facts = planDefaults(plan(#"{"cruise":\#(ranged),"altitude":\#(control("defaultMissionItemAltitude", 50, "m"))}"#))
        XCTAssertEqual(FactSlider(from: 1, to: 30, decimals: 1, hint: ""), facts.last?.slider)
        XCTAssertNil(facts.first?.slider)
    }

    func testASpecifiedMissionFlightSpeedDisablesTheCruiseAndHoverDefaultsLikeMissionDefaultsEditor() {
        let defaults = { (specified: Bool) in
            #"""
            {"altitude":\#(self.control("defaultMissionItemAltitude", 50, "m")),
                "cruise":\#(self.control("offlineEditingCruiseSpeed", 15, "m/s")),
                "hover":\#(self.control("offlineEditingHoverSpeed", 5, "m/s")),
                "ascent":\#(self.control("offlineEditingAscentSpeed", 5, "m/s")),
                "flightSpeed":{"available":true,"specified":\#(specified),"value":7.5}}
            """#
        }
        XCTAssertEqual([true, false, false, true], planDefaults(plan(defaults(true))).map(\.acceptsWrite))
        XCTAssertEqual([true, true, true, true], planDefaults(plan(defaults(false))).map(\.acceptsWrite))
    }
}
