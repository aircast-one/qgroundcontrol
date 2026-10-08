import XCTest
@testable import Aircast

final class AltitudeFrameTests: XCTestCase {
    private func item(_ frameText: String, text: String = "50.0 m", band: String = "", editUnits: String = "") -> MissionItem {
        MissionItem(
            index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: "Waypoint", selected: false, altitude: 50.0, kind: "waypoint",
            altitudeText: text, altitudeBandText: band, altitudeFrameText: frameText, altitudeEditUnits: editUnits
        )
    }

    func testAHeightAboveLaunchIsTheOneAnOperatorAlreadyAssumesSoItIsLeftBare() {
        XCTAssertEqual("50.0 m", altitudeWithFrame(item("")))
    }

    func testTheTwoFramesThatAreNotTheDefaultSaySo() {
        XCTAssertEqual("541 m AMSL", altitudeWithFrame(item("AMSL", text: "541 m")))
        XCTAssertEqual("50.0 m AGL", altitudeWithFrame(item("AGL")))
    }

    func testASurveyAndATakeoffBothReadingFiftyMetresAreNoLongerTheSameRow() {
        XCTAssertEqual("50.0 m", itemDetail(item("")))
        XCTAssertEqual("50.0 m AGL", itemDetail(item("AGL")))
    }

    func testABandTakesTheFrameTooRatherThanOnlySingleHeights() {
        XCTAssertEqual("0.0 m to 40.0 m AMSL", altitudeWithFrame(item("AMSL", text: "", band: "0.0 m to 40.0 m")))
    }

    func testAFrameTheCoreWithheldLeavesTheHeightUnlabelledRatherThanGuessingOne() {
        XCTAssertEqual("50.0 m", altitudeWithFrame(item("")))
        XCTAssertNil(altitudeWithFrame(item("AMSL", text: "")))
    }

    func testTheSummaryChipCarriesTheFrameTooBeingTheOtherPlaceAHeightIsSpelled() {
        let amsl = item("AMSL", text: "541 m")
        XCTAssertEqual("#2 at 541 m AMSL", selectionText(.Waypoint(index: 2), [amsl], [], []))
        XCTAssertEqual("#2 at 50.0 m", selectionText(.Waypoint(index: 2), [item("")], [], []))
    }

    func testTheFieldBeingTypedIntoNamesItsFrameBeingTheFurthestThingFromTheChip() {
        XCTAssertEqual("Alt m", altitudeFieldLabel(item("")))
        XCTAssertEqual("Alt m AMSL", altitudeFieldLabel(item("AMSL")))
        XCTAssertEqual("Alt m AGL", altitudeFieldLabel(item("AGL")))
        XCTAssertEqual("Alt m SEABED", altitudeFieldLabel(item("SEABED")))
    }

    func testAFrameNeitherHeadHasHeardOfIsShownNotSwallowedIntoTheDefault() {
        XCTAssertEqual("50.0 m SEABED", altitudeWithFrame(item("SEABED")))
    }

    func testTheFieldBeingTypedIntoNamesTheUnitQgcCookedTheNumberInto() {
        XCTAssertEqual("Alt ft", altitudeFieldLabel(item("", editUnits: "ft")))
        XCTAssertEqual("Alt ft AMSL", altitudeFieldLabel(item("AMSL", editUnits: "ft")))
    }

    func testACoreThatSaysNothingLeavesMetresWhichIsWhatTheBridgeTakes() {
        XCTAssertEqual("Alt m", altitudeFieldLabel(item("")))
    }
}
