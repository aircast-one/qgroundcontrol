import XCTest
@testable import Aircast

final class PostCalibrationTests: XCTestCase {
    func testOnlyACompletedArdupilotAccelOrCompassCalibrationAsksForAReboot() throws {
        XCTAssertEqual(postCalibrationPrompt(ACCEL_ROUTINE, CALIBRATION_COMPLETE, ACCEL_ROUTINE, false), "YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION.")
        XCTAssertTrue(try XCTUnwrap(postCalibrationPrompt(COMPASS_ROUTINE, "", COMPASS_ROUTINE, false)).hasPrefix("Shown in the indicator bars"))
        XCTAssertNil(postCalibrationPrompt(COMPASS_ROUTINE, CALIBRATION_COMPLETE, "", false), "a cancelled or failed run offers nothing")
        XCTAssertNil(postCalibrationPrompt("compassMot", CALIBRATION_COMPLETE, "", false), "CompassMot completes with the same help text but QGC opens no dialog")
        XCTAssertNil(postCalibrationPrompt(ACCEL_ROUTINE, CALIBRATION_COMPLETE, "", true))
        XCTAssertEqual(postCalibrationPrompt(COMPASS_ROUTINE, CALIBRATION_COMPLETE, "", true), "Reboot the vehicle prior to flight.")
        XCTAssertEqual(postCalibrationTitle(COMPASS_ROUTINE, true), "Compass calibration complete")
        XCTAssertEqual(postCalibrationTitle(COMPASS_ROUTINE, false), CALIBRATION_COMPLETE)
    }
}
