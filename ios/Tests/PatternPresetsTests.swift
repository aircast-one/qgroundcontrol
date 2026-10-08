import XCTest
@testable import Aircast

final class PatternPresetsTests: XCTestCase {
    func testPresetsShowForSurveysAndCorridorsOnly() {
        XCTAssertEqual("survey", presetKind(JSON.parse(#"{"presetKind":"survey"}"#)))
        XCTAssertNil(presetKind(JSON.parse(#"{"presetKind":null}"#)))
        XCTAssertNil(presetKind(JSON.parse("{}")))
        XCTAssertEqual(["Mapping 80m", "Inspection"], presetNames(JSON.parse(#"{"names":["Mapping 80m","Inspection"]}"#)))
    }

    func testPresetsComeFirstOnlyWhenDisplayPresetsTabFirstIsOn() {
        XCTAssertTrue(presetsShownFirst(JSON.parse(#"{"value":true}"#)))
        XCTAssertFalse(presetsShownFirst(JSON.parse(#"{"value":false}"#)))
        XCTAssertFalse(presetsShownFirst(JSON.parse(#"{"value":null}"#)))
        XCTAssertFalse(presetsShownFirst(JSON.parse("{}")))
    }
}
