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

    private func runs(_ html: String) -> [(String, URL?, InlinePresentationIntent?)] {
        let text = htmlAttributed(html)
        return text.runs.map { (String(text[$0.range].characters), $0.link, $0.inlinePresentationIntent) }
    }

    func testAParamAnchorBecomesALinkedRun() {
        let linked = runs(#"Check <a href="param://COM_ARM_WO_GPS">COM_ARM_WO_GPS</a> now"#)
        XCTAssertEqual(linked.map(\.0), ["Check ", "COM_ARM_WO_GPS", " now"])
        XCTAssertEqual(linked.map(\.1), [nil, URL(string: "param://COM_ARM_WO_GPS"), nil])
    }

    func testBoldAndItalicTagsSetTheIntents() {
        let styled = runs("<b>bold</b><i>italic</i><strong><em>both</em></strong>")
        XCTAssertEqual(styled.map(\.2), [.stronglyEmphasized, .emphasized, [.stronglyEmphasized, .emphasized]])
    }

    func testLineBreaksAndClosingBlocksStartNewLines() {
        XCTAssertEqual(String(htmlAttributed("one<br>two<p>three</p>four").characters), "one\ntwothree\nfour")
    }

    func testNamedAndNumericEntitiesAreDecoded() {
        XCTAssertEqual(String(htmlAttributed("5&deg; &#176; &#x2013; &ndash; &amp;lt; &bogus;").characters), "5\u{00b0} \u{00b0} \u{2013} \u{2013} &lt; &bogus;")
    }

    func testABareLessThanStaysInTheText() {
        XCTAssertEqual(String(htmlAttributed("alt < 5 m and <b>x</b> > 2").characters), "alt < 5 m and x > 2")
    }
}

