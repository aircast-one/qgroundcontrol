import XCTest
@testable import Aircast

final class OsdLabelTests: XCTestCase {
    func testTheFlyingReadingsShortenToDjisLettersAndAnythingElseKeepsItsName() {
        XCTAssertEqual(["Distance to home", "Alt (Rel)", "Ground Speed", "Climb Rate"].map(osdLabel), ["D", "H", "H.S", "V.S"])
        XCTAssertEqual(osdLabel("Flight time"), "FLIGHT TIME")
        XCTAssertEqual(osdLabel("From you"), "D.OP")
        XCTAssertEqual(["distanceToHome", "altitudeRelative", "groundSpeed", "climbRate"].map(osdLabel), ["D", "H", "H.S", "V.S"])
    }
}

final class DjiChromeTests: XCTestCase {
    func testTheStatusTitleSplitsIntoPlainModeTextAndANoteForThePill() {
        XCTAssertEqual(osdModeText("Manual · Not fully ready"), "Manual")
        XCTAssertEqual(osdStatusNote("Manual · Not fully ready"), "Not fully ready")
        XCTAssertEqual(osdStatusNote("Manual"), nil)
    }

    func testSpeedsGoOnTheSmallTopRowAndTheRestOnTheLargeOne() {
        XCTAssertEqual(["Ground Speed", "Climb Rate", "Alt (Rel)", "Distance to home"].map(osdIsSpeed), [true, true, false, false])
    }

    func testTheBatteryRingReadsThePercentageOutOfTheCellText() {
        XCTAssertEqual(batteryPercent("B1 53%"), 53)
        XCTAssertEqual(batteryPercent("No battery"), nil)
    }

    func testTheMiniMapOpensAsAThumbnailUnlessThePilotChoseOtherwise() {
        XCTAssertEqual(miniMapNamed(nil), MiniMap.Thumb)
        XCTAssertEqual(miniMapNamed("Compass"), MiniMap.Compass)
        XCTAssertEqual(miniMapNamed("Unknown"), MiniMap.Thumb)
    }
}

final class FlightTimeTests: XCTestCase {
    func testFlightTimeReadsAsDjisMinutesAndSeconds() {
        XCTAssertEqual(flightTimeText(nil), "00'00\"")
        XCTAssertEqual(flightTimeText(0.0), "00'00\"")
        XCTAssertEqual(flightTimeText(75.4), "01'15\"")
        XCTAssertEqual(flightTimeText(4325.0), "72'05\"")
    }
}

final class FlightTimeReadTests: XCTestCase {
    func testTheSecondsAreTheValueOfTheFactTheCoreAnswersWith() {
        XCTAssertEqual(flightTimeSeconds(JSON.parse(#"{"kind":"fact","value":10.325}"#)), 10.325)
        XCTAssertEqual(flightTimeSeconds(JSON.parse(#"{"kind":"fact","value":null}"#)), nil)
    }
}

final class RailLabelTests: XCTestCase {
    func testRailLabelsDropTheHoldInstruction() {
        XCTAssertEqual(["Hold to land", "Return", "Hold to take off"].map(railLabel), ["Land", "Return", "Take off"])
    }
}
