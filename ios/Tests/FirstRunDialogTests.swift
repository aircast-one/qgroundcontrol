import XCTest
@testable import Aircast

final class FirstRunDialogTests: XCTestCase {
    func testThePromptShowsUntilTheCoreSaysItWasShown() throws {
        let prompt = try XCTUnwrap(firstRun(JSON.parse(
            #"{"show":true,"title":"Preferences","vehicleHeading":"Vehicle Preferences","vehicleDescription":"d","vehiclePreferences":[{"name":"preferredFirmwareClass","label":"Preferred Firmware","control":"choice","path":"settings.appSettings.preferredFirmwareClass"}],"unitsHeading":"Measurement Units","unitsDescription":"u"}"#
        )))
        XCTAssertEqual(prompt.title, "Preferences")
        XCTAssertEqual(prompt.preferences.map(\.title), ["Preferred Firmware"])
        XCTAssertNil(firstRun(JSON.parse(#"{"show":false}"#)))
    }

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

    private var shown: [Fact] {
        ["horizontalDistanceUnits", "verticalDistanceUnits", "areaUnits", "speedUnits", "temperatureUnits", "weightUnits", "customUnits"].map(fact)
    }

    func testTheFiveUnitRowsAlwaysShowLikeInitialSetupPromptsRepeater() {
        XCTAssertEqual(firstRunUnitRows(shown).map(\.name), ["horizontalDistanceUnits", "verticalDistanceUnits", "areaUnits", "speedUnits", "temperatureUnits"])
    }

    func testImperialWritesFeetSquareFeetFeetPerSecondAndFahrenheitLikeChangeSystemOfUnits() {
        let rows = firstRunUnitRows(shown)
        XCTAssertEqual(firstRunSystemWrites(false, rows).map(\.1), [0, 0, 0, 0, 1])
        XCTAssertEqual(firstRunSystemWrites(true, rows).map(\.1), [1, 1, 1, 1, 0])
        XCTAssertEqual(firstRunSystemWrites(true, rows).first?.0, "settings.unitsSettings.horizontalDistanceUnits")
    }

    func testHiddenUnitsAreNotWritten() {
        XCTAssertEqual(firstRunSystemWrites(true, firstRunUnitRows([fact("speedUnits")])).map(\.0), ["settings.unitsSettings.speedUnits"])
    }

    func testMetricIsChosenOnlyWhileHorizontalDistanceIsInMeters() {
        XCTAssertEqual(firstRunSystemIndex(1.0), 0)
        XCTAssertEqual(firstRunSystemIndex(0.0), 1)
        XCTAssertEqual(firstRunSystemIndex(.nan), 1)
        XCTAssertEqual(FIRST_RUN_SYSTEMS, ["Metric System", "Imperial System"])
    }
}
