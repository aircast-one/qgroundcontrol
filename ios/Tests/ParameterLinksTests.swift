import XCTest
@testable import Aircast

final class ParameterLinksTests: XCTestCase {
    func testAParamLinkNamesTheParameterAndOtherLinksAreLeftToTheBrowser() {
        XCTAssertEqual(paramLinkName("param://COM_ARM_WO_GPS"), "COM_ARM_WO_GPS")
        XCTAssertNil(paramLinkName("param://"))
        XCTAssertNil(paramLinkName("https://docs.px4.io"))
    }

    func testAReadOnlyParameterSaysSoUntilForceEditIsOn() {
        XCTAssertNil(forceEditNote(false, false))
        XCTAssertEqual(forceEditNote(true, false), READ_ONLY_NOTE)
        XCTAssertEqual(forceEditNote(true, true), FORCE_EDIT_NOTE)
    }

    func testResetOffersTheDefaultOnlyWhenTheMetadataHasOne() {
        XCTAssertEqual(parameterDefault(JSON.parse(#"{"defaultValueAvailable":true,"defaultValue":20}"#)), .number(20))
        XCTAssertNil(parameterDefault(JSON.parse(#"{"defaultValueAvailable":false,"defaultValue":20}"#)))
        XCTAssertNil(parameterDefault(JSON.parse(#"{"defaultValueAvailable":true,"defaultValue":null}"#)))
    }

    func testManualEntryDropsTheChoiceLists() {
        let fact = Fact(path: "p", name: "MODE", description: "", units: "", valueString: "1", value: .number(1), enumStrings: ["A", "B"], enumValues: ["0", "1"], enumIndex: 1, isBool: false, isString: false, readOnly: false)
        XCTAssertFalse(manualEntryFact(fact).isEnum)
    }

    func testValueDetailsReadTheControlsDescriptionAndDefaultLikeTheLandingAltitudeDialog() throws {
        let control = JSON.parse(#"{"path":"item.finalApproachAltitude","name":"FinalApproachAltitude","label":"Altitude","valueDetails":"Altitude to begin landing approach from.","units":"m","valueString":"40.0","value":40.0,"control":"number","readOnly":false,"minimumText":null,"maximumText":null,"defaultText":"40.0"}"#)
        let fact = try XCTUnwrap(factFromControl(control))
        XCTAssertEqual(fact.valueDetails, "Altitude to begin landing approach from.")
        XCTAssertEqual(valueDetailsNotes(fact), ["Altitude to begin landing approach from.", "default 40.0 m"])
    }
}
