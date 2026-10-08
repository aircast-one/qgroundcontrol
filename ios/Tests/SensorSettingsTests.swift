import XCTest
@testable import Aircast

final class SensorSettingsTests: XCTestCase {
    func testCompassesAndPrioritySlotsReadFromTheCore() throws {
        XCTAssertNil(sensorSettings(JSON.parse(#"{"available":false}"#)))
        let read = try XCTUnwrap(sensorSettings(JSON.parse(#"""
            {"available":true,"boardRotation":null,"priorities":["Priority 1","Priority 2","Priority 3","Not Set"],
            "compasses":[{"index":0,"label":"Compass 1 (primary, external)","device":"IST8310 (I2C1)","use":null,"priority":0,"orientation":null,"orientationTitle":"Orientation"}],"boardTitle":"Autopilot Rotation","compassesWhileCalibrating":true,
            "helpSet":"a","helpCal":"b","simpleAccelHelp":"c","declination":null}
            """#)))
        XCTAssertEqual(read.compasses, [CompassSettings(index: 0, label: "Compass 1 (primary, external)", device: "IST8310 (I2C1)", use: nil, priority: 0, orientation: nil, orientationTitle: "Orientation")])
        XCTAssertEqual(read.priorities[3], "Not Set")
    }

    func testPx4MagsCarryTheirOwnTitlesAndStayOutOfTheCalibrationDialog() throws {
        let read = try XCTUnwrap(sensorSettings(JSON.parse(#"""
            {"available":true,"boardRotation":null,"boardTitle":"Autopilot Orientation","compassesWhileCalibrating":false,"priorities":[],
            "compasses":[{"index":1,"label":"Mag 1","device":"","use":null,"priority":null,"orientation":null,"orientationTitle":"Mag 1 Orientation"}],
            "helpSet":"a","helpCal":"b","simpleAccelHelp":"","declination":null}
            """#)))
        XCTAssertEqual(read.boardTitle, "Autopilot Orientation")
        XCTAssertEqual(read.compasses.first?.orientationTitle, "Mag 1 Orientation")
        XCTAssertEqual(read.compasses.count, 1)
        XCTAssertFalse(read.compassesWhileCalibrating)
    }
}
