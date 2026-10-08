import XCTest
@testable import Aircast

final class GuidedSpeedTests: XCTestCase {
    func testTheCorePicksTheCommandSoTheHeadNeverChoosesAirspeedOrGroundSpeed() {
        let multirotor = guidedSpeed(JSON.parse(#"""
            {"available":true,"label":"Speed","unit":"m/s",
            "command":"guidedModeChangeGroundSpeedMetersSecond",
            "initial":5.0,"minimum":1.0,"maximum":20.0}
            """#))!
        let forward = guidedSpeed(JSON.parse(#"""
            {"available":true,"label":"Airspeed","unit":"m/s",
            "command":"guidedModeChangeEquivalentAirspeedMetersSecond",
            "initial":15.0,"minimum":5.0,"maximum":30.0}
            """#))!
        XCTAssertEqual(multirotor.command, "guidedModeChangeGroundSpeedMetersSecond")
        XCTAssertEqual(forward.command, "guidedModeChangeEquivalentAirspeedMetersSecond")
        XCTAssertEqual(multirotor.label, "Speed")
        XCTAssertEqual(forward.label, "Airspeed")
    }

    func testAVehicleWithNoSpeedCommandOffersNoDialog() {
        let none = guidedSpeed(JSON.parse(#"""
            {"available":true,"label":null,"unit":"m/s","command":null,
            "initial":null,"minimum":null,"maximum":null}
            """#))!
        XCTAssertNil(none.command)
        XCTAssertEqual(none.label, "Speed")
        XCTAssertNil(speedReading(none)?.range)
    }

    func testAUsableRangeNeedsACommandAndBothBounds() {
        let usable = speedReading(guidedSpeed(JSON.parse(#"""
            {"available":true,"command":"guidedModeChangeGroundSpeedMetersSecond",
            "label":"Speed","unit":"m/s","minimum":1.0,"maximum":20.0,"initial":5.0}
            """#)))
        XCTAssertEqual(usable?.range, 1.0...20.0)
        XCTAssertNil(speedReading(nil))
        XCTAssertNil(guidedSpeed(JSON.parse(#"{"available":false}"#)))
    }

    func testTheArgumentPathCarriesTheTargetAndNoSeparators() {
        XCTAssertEqual(guidedSpeedPath(7.5), "view.guidedSpeed(7.50)")
        XCTAssertFalse(guidedSpeedPath(7.5).contains(","))
    }
}
