import XCTest
@testable import Aircast

final class GuidedAltitudeTests: XCTestCase {
    func testTheServedReadingCarriesTheSentenceAndTheMetricDelta() {
        let reading = guidedAltitude(JSON.parse(#"""
            {"available":true,"label":"Alt (Rel)","unit":"m","current":25.0,
            "currentMeters":25.0,"minimum":0.0,"maximum":121.0,"target":68.5,
            "targetMeters":68.5,"delta":43.5,"deltaMeters":43.5,"sends":true,
            "sentence":"The aircraft will climb 43.5 m to 68.5 m."}
            """#))!
        XCTAssertEqual(reading.sentence, "The aircraft will climb 43.5 m to 68.5 m.")
        XCTAssertEqual(reading.deltaMeters, 43.5, accuracy: 1e-6)
        XCTAssertTrue(reading.sends)
    }

    func testAVehicleWithNoAltitudeYetReadsAsAbsentNotAsZero() {
        let reading = guidedAltitude(JSON.parse(#"""
            {"available":true,"label":"Alt (Rel)","unit":"m","current":null,
            "currentMeters":null,"minimum":null,"maximum":null}
            """#))!
        XCTAssertNil(reading.current)
        XCTAssertNil(reading.minimum)
        XCTAssertNil(reading.maximum)
        XCTAssertNil(altitudeReading(reading)?.range)
    }

    func testAnUnavailableViewOffersNothing() {
        XCTAssertNil(guidedAltitude(nil))
        XCTAssertNil(guidedAltitude(JSON.parse(#"{"available":false}"#)))
        XCTAssertNil(altitudeReading(nil))
    }

    func testTheArgumentPathCarriesTheTargetAndNoStraySeparators() {
        XCTAssertEqual(guidedAltitudePath(68.5), "view.guidedAltitude(68.50)")
        XCTAssertFalse(guidedAltitudePath(68.5).contains(","))
    }

    func testAPauseAsksTheCoreForThePauseIntent() {
        XCTAssertEqual(guidedAltitudePath(25.0, pause: true), "view.guidedAltitude(25.00,pause)")
    }

    func testTheCommandCarriesTheChangeNeverTheHeightTheOperatorTyped() {
        let reading = guidedAltitude(JSON.parse(#"""
            {"kind":"object","class":"GuidedAltitude","available":true,"label":"Altitude","unit":"m",
            "current":25.0,"minimum":0.0,"maximum":120.0,"target":68.5,
            "targetMeters":68.5,"delta":43.5,"deltaMeters":43.5,"sends":true,
            "pause":false,"sentence":"The aircraft will climb 43.5 m to 68.5 m."}
            """#))!
        XCTAssertEqual(altitudeDelta(reading)!, 43.5, accuracy: 1e-9)
    }

    func testAChangeTooSmallToMatterSendsNothingAtAll() {
        let standing = guidedAltitude(JSON.parse(#"""
            {"kind":"object","class":"GuidedAltitude","available":true,"label":"Altitude","unit":"m",
            "current":25.0,"minimum":0.0,"maximum":120.0,"target":25.0,
            "targetMeters":25.0,"delta":0.0,"deltaMeters":0.0,"sends":false,
            "pause":false,"sentence":"The aircraft is already at 25 m and will not move."}
            """#))!
        XCTAssertNil(altitudeDelta(standing))
    }
}

final class RangeLabelTests: XCTestCase {
    func testASliderSaysWhatItsEndsAre() {
        XCTAssertEqual(rangeLabel(3.0, 121.92, "m"), "3.0 to 121.9 m")
        XCTAssertEqual(rangeLabel(0.1, 5.0, "m/s"), "0.1 to 5.0 m/s")
    }

    func testARangeWithNoUnitStillReads() {
        XCTAssertEqual(rangeLabel(1.0, 8.0, ""), "1.0 to 8.0")
    }

    func testARangeThatIsNotARangeSaysNothing() {
        XCTAssertNil(rangeLabel(nil, 5.0, "m"))
        XCTAssertNil(rangeLabel(1.0, nil, "m"))
        XCTAssertNil(rangeLabel(5.0, 5.0, "m"))
        XCTAssertNil(rangeLabel(9.0, 5.0, "m"))
    }

    func testAnAltitudeAHairBelowLaunchReadsAsZeroNotAsMinusZero() {
        XCTAssertEqual(rangeLabel(-0.03, 60.0, "m"), "0.0 to 60.0 m")
    }
}
