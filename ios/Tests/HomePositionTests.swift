import XCTest
@testable import Aircast

final class HomePositionTests: XCTestCase {
    private func coordinate(_ latitude: Double, _ longitude: Double, valid: Bool = true) -> JSON {
        JSON.parse(#"{"kind":"coordinate","valid":\#(valid),"latitude":\#(latitude),"longitude":\#(longitude)}"#)
    }

    func testAValidCoordinateIsRead() {
        let point = coordinateOf(coordinate(-35.363262, 149.165237))

        XCTAssertEqual(point?.latitude ?? .nan, -35.363262, accuracy: 1e-9)
        XCTAssertEqual(point?.longitude ?? .nan, 149.165237, accuracy: 1e-9)
    }

    func testACoordinateTheVehicleHasNotEstablishedIsIgnored() {
        XCTAssertNil(coordinateOf(coordinate(0.0, 0.0, valid: false)))
        XCTAssertNil(coordinateOf(coordinate(-35.36, 149.16, valid: false)))
    }

    func testNullIslandIsNotAHomePositionEvenWhenFlaggedValid() {
        XCTAssertNil(coordinateOf(coordinate(0.0, 0.0)))
    }

    func testAnAbsentOrUnrelatedPayloadYieldsNothing() {
        XCTAssertNil(coordinateOf(nil))
        XCTAssertNil(coordinateOf(JSON.parse(#"{"kind":"null"}"#)))
    }
}
