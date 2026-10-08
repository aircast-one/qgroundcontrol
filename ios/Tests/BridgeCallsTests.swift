import XCTest
@testable import Aircast

final class BridgeCallsTests: XCTestCase {
    func testACoordinateCarriesAllThreeKeysBecauseTheBridgeNeedsLatitudeAndLongitude() {
        XCTAssertEqual(coordinateJson(41.5, 44.25), ["latitude": 41.5, "longitude": 44.25, "altitude": 0])
        XCTAssertEqual(JSON.encode(coordinateJson(41.5, 44.25)), #"{"altitude":0,"latitude":41.5,"longitude":44.25}"#)
    }

    func testATrackPointMakesTheSameCoordinateAsItsTwoNumbers() {
        XCTAssertEqual(coordinateJson(TrackPoint(latitude: 41.5, longitude: 44.25)), coordinateJson(41.5, 44.25))
    }

    func testAPropertyWriteWrapsItsValueWhichABareCoordinateDoesNot() {
        XCTAssertEqual(JSON.encode(["value": coordinateJson(41.5, 44.25)]), #"{"value":{"altitude":0,"latitude":41.5,"longitude":44.25}}"#)
        XCTAssertEqual(JSON.encode(["value": 50.5]), #"{"value":50.5}"#)
    }

    func testADoubleIsWrittenInAFormTheBridgeParsesWhateverThePhonesLocale() {
        XCTAssertEqual(JSON.encode(coordinateJson(-0.5, 179.125)), #"{"altitude":0,"latitude":-0.5,"longitude":179.125}"#)
    }
}
