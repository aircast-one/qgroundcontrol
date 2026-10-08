import XCTest
@testable import Aircast

final class UnitsSectionTests: XCTestCase {
    private func fact(_ name: String) -> Fact {
        Fact(
            path: "settings.unitsSettings.\(name)",
            name: name,
            description: name,
            units: "",
            valueString: "0",
            value: .number(0),
            enumStrings: ["A", "B"],
            enumIndex: 0,
            isBool: false,
            isString: false,
            readOnly: false
        )
    }

    private var all: [Fact] { (PRESET_UNIT_FACTS + ["weightUnits", "customUnits"]).map(fact) }

    func testAPresetHidesTheMeasurementsItAlreadyDecides() {
        XCTAssertEqual(unitRowsFor(0, all).map(\.name), [])
    }

    func testCustomShowsEveryMeasurement() {
        XCTAssertEqual(unitRowsFor(UNIT_SYSTEM_CUSTOM, all).map(\.name), PRESET_UNIT_FACTS)
    }

    func testTheCustomUnitsFlagIsNeverARowOfItsOwn() {
        [0, 1, UNIT_SYSTEM_CUSTOM].forEach { system in
            XCTAssertFalse(unitRowsFor(system, all).contains { $0.name == "customUnits" })
        }
    }

    func testWeightIsNeverARowAsGeneralSettingsListsOnlyFiveUnitsAndWeightFollowsTheLocale() {
        [0, 1, UNIT_SYSTEM_CUSTOM].forEach { system in
            XCTAssertFalse(unitRowsFor(system, all).contains { $0.name == "weightUnits" })
        }
    }

    func testTheNoteNamesTheSystemInForce() {
        XCTAssertTrue(unitSystemNote(0).contains("Metric"))
        XCTAssertTrue(unitSystemNote(1).contains("Imperial"))
        XCTAssertEqual(unitSystemNote(UNIT_SYSTEM_CUSTOM), "Each measurement is set on its own below.")
    }

    func testASystemIndexTheBridgeShouldNeverSendIsTreatedAsCustom() {
        [-1, 3, 99].forEach { rogue in
            XCTAssertEqual(unitSystemLabel(rogue), "Custom")
            XCTAssertEqual(unitSystemNote(rogue), "Each measurement is set on its own below.")
            XCTAssertEqual(unitRowsFor(rogue, all).map(\.name), PRESET_UNIT_FACTS)
        }
    }

    func testUnitRowsComeFromTheUnitsGroupOfTheGeneralSettingsPage() {
        let page = JSON.parse(
            #"""
            {"sections":[
                 {"group":"appSettings","subsections":[{"title":"","controls":[{"name":"audioMuted","label":"Mute","control":"toggle"}]}]},
                 {"group":"unitsSettings","subsections":[
                   {"title":"","controls":[{"name":"horizontalDistanceUnits","label":"Distance","control":"choice","options":[{"label":"Feet","raw":"0"},{"label":"Meters","raw":"1"}],"display":"Meters"}]},
                   {"title":"Other","controls":[{"name":"temperatureUnits","label":"Temperature","control":"choice","options":[],"display":""}]}
                 ]}]}
            """#
        )
        let facts = unitFacts(page)
        XCTAssertEqual(facts.map(\.name), ["horizontalDistanceUnits", "temperatureUnits"])
        XCTAssertEqual(facts.first?.enumIndex, 1)
        XCTAssertEqual(unitFacts(nil).map(\.name), [])
    }
}
