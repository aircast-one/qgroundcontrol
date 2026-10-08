import XCTest
@testable import Aircast

final class AttitudeInstrumentTests: XCTestCase {
    func testReadsTheAnglesAndHidesPointersTheCoreWithholds() throws {
        let reading = try XCTUnwrap(attitude(JSON.parse(
            #"{"available":true,"roll":-12.5,"pitch":4,"heading":7.4,"headingText":"7°","courseOverGround":null,"headingToHome":190,"headingToNextWaypoint":null,"noseUp":false}"#
        )))
        XCTAssertEqual(reading.roll, -12.5)
        XCTAssertEqual(reading.headingText, "7\u{00b0}")
        XCTAssertNil(reading.courseOverGround)
        XCTAssertEqual(reading.headingToHome, 190)
        XCTAssertFalse(reading.noseUp)
    }

    func testNoVehicleReadsAsNothingAndThePanelDrawsTheBareDialAtNorth() {
        XCTAssertNil(attitude(JSON.parse(#"{"available":false}"#)))
        XCTAssertNil(attitude(nil))
        XCTAssertEqual(NO_VEHICLE_ATTITUDE.heading, 0)
        XCTAssertEqual(NO_VEHICLE_ATTITUDE.headingText, "")
    }

    func testPitchMovesTheHorizonByTheSpanQgcUses() {
        XCTAssertEqual(pitchOffset(45, 20), 40)
        XCTAssertEqual(pitchOffset(0, 20), 0)
    }
}
