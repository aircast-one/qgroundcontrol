import XCTest
@testable import Aircast

final class AltitudeEntryTests: XCTestCase {
    func testAPlainNumberIsAnAltitude() throws {
        XCTAssertEqual(try XCTUnwrap(parsedAltitude("47")), 47, accuracy: 1e-9)
        XCTAssertEqual(try XCTUnwrap(parsedAltitude(" 47.5 ")), 47.5, accuracy: 1e-9)
        XCTAssertEqual(try XCTUnwrap(parsedAltitude("0")), 0, accuracy: 1e-9)
    }

    func testNonsenseAndHalfTypedValuesAreRefusedRatherThanSent() {
        XCTAssertNil(parsedAltitude(""))
        XCTAssertNil(parsedAltitude("."))
        XCTAssertNil(parsedAltitude("4x"))
    }

    func testBelowTheLaunchPointAndACommaDecimalAreAltitudesAsSimpleMissionItemSetsNoMinimum() throws {
        XCTAssertEqual(try XCTUnwrap(parsedAltitude("-5")), -5, accuracy: 1e-9)
        XCTAssertEqual(try XCTUnwrap(parsedAltitude("45,5")), 45.5, accuracy: 1e-9)
        XCTAssertNil(parsedAltitude("1,000.5"))
        XCTAssertNil(parsedAltitude("1,000,000"))
    }

    func testTheFieldKeepsTheDecimalsQGCShowsSoDoneOnAnUntouchedFieldWritesTheSameHeightBack() {
        XCTAssertEqual(altitudeFieldText(75, 1), "75")
        XCTAssertEqual(altitudeFieldText(45.5, 1), "45.5")
        XCTAssertEqual(altitudeFieldText(164.0420, RALLY_ALTITUDE_DECIMALS), "164.04")
        XCTAssertEqual(altitudeFieldText(.nan, 1), "")
        XCTAssertEqual(altitudeFieldText(-0.04, 1), "0")
    }

    func testASurveyKeepsCameraCalcsTenthOfAMetreFloorAboveTheSurface() throws {
        XCTAssertNil(parsedSurfaceDistance("0", 1))
        XCTAssertEqual(try XCTUnwrap(parsedSurfaceDistance("0,1", 1)), 0.1, accuracy: 1e-9)
        XCTAssertNil(parsedSurfaceDistance("0.2", metresPerUnit("ft")))
    }

    func testARallyCoordinateIsTypedWithinTheGlobeLikeRallyPointsTextFieldFacts() throws {
        XCTAssertEqual(
            [parsedCoordinate("47.39", LATITUDE_LIMIT), parsedCoordinate("91", LATITUDE_LIMIT), parsedCoordinate("-180", LONGITUDE_LIMIT), parsedCoordinate("x", LONGITUDE_LIMIT)],
            [47.39, nil, -180, nil]
        )
        XCTAssertEqual(try XCTUnwrap(parsedCoordinate("-33,87", LATITUDE_LIMIT)), -33.87, accuracy: 1e-9)
    }
}
