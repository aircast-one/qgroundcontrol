import XCTest
@testable import Aircast

final class ApmSubMotorsScreenTests: XCTestCase {
    func testMotorsTheirDirectionsAndTheTestGateComeFromTheCore() throws {
        let read = try XCTUnwrap(subMotors(JSON.parse(#"""
            {"available":true,"armed":true,"detecting":false,"canRunManualTest":true,
            "motors":[{"motor":1,"reversed":false},{"motor":2,"reversed":true}],"warning":"w","offersAutoDetect":true,"autoDetectHelp":"h","detectionMessages":"Thruster 1 ok\n"}
            """#)))
        XCTAssertEqual(read.motors, [SubMotor(motor: 1, reversed: false), SubMotor(motor: 2, reversed: true)])
        XCTAssertTrue(read.canRunManualTest)
        XCTAssertTrue(read.offersAutoDetect)
        XCTAssertEqual(read.detectionMessages, "Thruster 1 ok\n")
        XCTAssertNil(subMotors(JSON.parse(#"{"available":false}"#)))
    }

    func testTheFramePictureShowsOnlyForFramesApmSubMotorDisplayDraws() {
        let frames = { (selected: Int?) in SubFrames(frames: [], selected: selected, confirmFirst: false, loading: false, loadError: "") }
        XCTAssertEqual(motorDisplayFrame(frames(1)), 1)
        XCTAssertEqual(motorDisplayFrame(frames(5)), 5)
        XCTAssertNil(motorDisplayFrame(frames(6)))
        XCTAssertNil(motorDisplayFrame(frames(3)))
        XCTAssertNil(motorDisplayFrame(frames(nil)))
        XCTAssertNil(motorDisplayFrame(nil))
    }
}
