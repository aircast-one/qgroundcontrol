import XCTest
@testable import Aircast

final class FastCompassTests: XCTestCase {
    private let withGcs = fastCompass(JSON.parse(#"{"invocation":"sensorsCal.calibrateCompassNorth","help":"h","vehicleHasPosition":false,"gcsPosition":{"valid":true,"latitude":41.7,"longitude":44.8}}"#))!
    private let withoutGcs = fastCompass(JSON.parse(#"{"invocation":"sensorsCal.calibrateCompassNorth","help":"h","vehicleHasPosition":true,"gcsPosition":{"valid":false,"latitude":null,"longitude":null}}"#))!

    func testAValidGcsPositionIsUsedUnlessTheOperatorUnticksItLikeQgc() {
        let choice = initialFastCompassChoice(withGcs).with { $0.enabled = true }
        XCTAssertTrue(choice.useGcs)
        XCTAssertEqual(fastCompassArguments(withGcs, choice), [.number(41.7), .number(44.8)])
        XCTAssertEqual(
            fastCompassArguments(withGcs, choice.with { $0.useGcs = false; $0.latitude = "1.5"; $0.longitude = "2.5" }),
            [.number(1.5), .number(2.5)]
        )
    }

    func testWithoutAGcsPositionTheTypedCoordinatesGoToTheCoreWhichRejectsGarbage() {
        let choice = initialFastCompassChoice(withoutGcs).with { $0.enabled = true }
        XCTAssertFalse(choice.useGcs)
        XCTAssertNil(withoutGcs.gcsLatitude)
        XCTAssertEqual(fastCompassArguments(withoutGcs, choice), [.number(0), .number(0)])
        XCTAssertEqual(fastCompassArguments(withoutGcs, choice.with { $0.latitude = "x" }), [.string("x"), .number(0)])
    }

    func testTheFlyMapCentreStandsInForThePositionWhenTickedLikeApmSensorsComponent() {
        let choice = initialFastCompassChoice(withoutGcs, mapPosition: TrackPoint(latitude: 41.71, longitude: 44.79)).with { $0.enabled = true }
        XCTAssertEqual(fastCompassArguments(withoutGcs, choice), [.number(0), .number(0)])
        XCTAssertEqual(fastCompassArguments(withoutGcs, choice.with { $0.useMap = true }), [.number(41.71), .number(44.79)])
        XCTAssertEqual(fastCompassArguments(withoutGcs, choice.with { $0.useMap = true; $0.mapPosition = nil }), [.number(0), .number(0)])
    }

    func testPx4ServesNoFastCompass() {
        XCTAssertNil(fastCompass(nil))
    }
}
