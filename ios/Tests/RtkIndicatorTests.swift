import XCTest
@testable import Aircast

final class RtkIndicatorTests: XCTestCase {
    func testADisconnectedBaseShowsNothing() {
        XCTAssertNil(rtkStatus(JSON.parse(#"{"connected":false}"#)))
        XCTAssertNil(rtkStatus(nil))
    }

    func testASurveyInProgressReadsLikeQgcsRtkSection() throws {
        let status = try XCTUnwrap(rtkStatus(JSON.parse(#"{"connected":true,"active":true,"valid":false,"numSatellites":17,"currentDuration":42.0,"currentAccuracy":3.456,"currentAccuracyText":"11.3 ft"}"#)))
        XCTAssertEqual(rtkHeadline(status), "Survey-in Active")
        XCTAssertEqual(rtkRows(status).map { [$0.0, $0.1] }, [["Satellites", "17"], ["Duration", "42 s"], ["Current Accuracy", "11.3 ft"]])
    }

    func testAFinishedSurveyStreamsAndHidesAnAccuracyItHasNotReported() throws {
        let status = try XCTUnwrap(rtkStatus(JSON.parse(#"{"connected":true,"active":false,"valid":true,"numSatellites":null,"currentDuration":null,"currentAccuracy":null}"#)))
        XCTAssertEqual(rtkHeadline(status), "RTK Streaming")
        XCTAssertEqual(rtkRows(status).map { [$0.0, $0.1] }, [["Satellites", ""], ["Duration", "0 s"]])
    }

    func testTheSurveyedPositionIsSavedAsTheFixedBaseOnlyOnceTheSurveyIsValid() throws {
        let surveying = RtkStatus(active: true, valid: false, satellites: 9, durationS: 10.0, accuracyM: 3.0, latitude: 47.1, longitude: 8.5, altitudeM: 400.0)
        XCTAssertNil(basePositionWrites(surveying))
        var valid = surveying
        valid.valid = true
        let writes = try XCTUnwrap(basePositionWrites(valid))
        XCTAssertEqual(writes.map(\.0), [
            "settings.rtkSettings.fixedBasePositionLatitude",
            "settings.rtkSettings.fixedBasePositionLongitude",
            "settings.rtkSettings.fixedBasePositionAltitude",
            "settings.rtkSettings.fixedBasePositionAccuracy",
        ])
        XCTAssertEqual(writes.map(\.1), [47.1, 8.5, 400.0, 3.0])
        var unplaced = valid
        unplaced.latitude = nil
        XCTAssertNil(basePositionWrites(unplaced))
    }
}
