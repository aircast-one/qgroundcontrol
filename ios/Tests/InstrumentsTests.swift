import XCTest
@testable import Aircast

final class InstrumentsTests: XCTestCase {
    func testAReadingJoinsItsUnitsAndAUnitlessOneDoesNot() {
        let shown = instruments(JSON.parse(#"{"items":[{"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},{"label":"Heading","value":"356","units":"","missing":false}]}"#))
        XCTAssertEqual(shown.map(\.reading), ["25.0 m", "356"])
        XCTAssertEqual(shown.map(\.label), ["Alt (Rel)", "Heading"])
    }

    func testAMissingInstrumentIsLeftOutRatherThanShownBlank() {
        let shown = instruments(JSON.parse(#"{"items":[{"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},{"label":"Distance to Home","value":"","units":"","missing":true}]}"#))
        XCTAssertEqual(shown.map(\.label), ["Alt (Rel)"])
    }

    func testNoViewAtAllIsAnEmptyRowNotACrash() {
        XCTAssertEqual(instruments(nil), [])
        XCTAssertEqual(instruments(JSON.parse("{}")), [])
    }
}

final class InstrumentMissingTests: XCTestCase {
    private func item(_ reason: String) -> [Instrument] {
        instruments(JSON.parse(#"{"items":[{"label":"Alt (Rel)","value":"25.0","units":"m","missing":false},{"label":"Distance to Home","value":"","units":"","missing":true,"missingReason":"# + reason + "}]}"))
    }

    func testAFactTheVehicleHasNotReportedYetKeepsItsPlaceAndWaits() {
        let shown = item(#""notReported""#)
        XCTAssertEqual(shown.map(\.label), ["Alt (Rel)", "Distance to Home"])
        XCTAssertEqual(shown[1].reading, "\u{2014}")
    }

    func testAFactThisVehicleDoesNotHaveIsLeftOutBecauseItWillNeverFillIn() {
        XCTAssertEqual(item(#""noSuchFact""#).map(\.label), ["Alt (Rel)"])
    }

    func testAMissingItemWithNoReasonIsLeftOutRatherThanWaitingForever() {
        XCTAssertEqual(item("null").map(\.label), ["Alt (Rel)"])
    }
}
