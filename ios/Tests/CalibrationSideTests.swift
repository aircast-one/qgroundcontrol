import XCTest
@testable import Aircast

final class CalibrationSideTests: XCTestCase {
    private func side(_ stage: String, rotate: Bool = false) -> CalibrationSide {
        CalibrationSide(key: "NoseDown", title: "Nose down", visible: true, stage: stage, rotate: rotate)
    }

    func testEachSideShowsVehicleRotationCalsPictureAndState() {
        XCTAssertEqual(sideImage("NoseDown", false), "cal_vehicle_nose_down")
        XCTAssertEqual(sideImage("NoseDown", true), "cal_vehicle_nose_down_rotate")
        XCTAssertEqual(sideImage("Down", false), "cal_vehicle_down")
        XCTAssertEqual(sideStateText(side("inProgress", rotate: true)), "Rotate")
        XCTAssertEqual(sideStateText(side("inProgress")), "Hold still")
        XCTAssertEqual(sideStateText(side("done")), "Done")
        XCTAssertEqual(sideStateText(side("idle")), "Pending")
    }
}
