import XCTest
@testable import Aircast

final class OperatorPositionTests: XCTestCase {
    func testAUsableFixIsAPointOnTheMap() {
        let view = JSON.parse(#"{"usable":true,"latitude":41.6983,"longitude":44.85148}"#)
        XCTAssertEqual(operatorPoint(view), TrackPoint(latitude: 41.6983, longitude: 44.85148))
        XCTAssertEqual(operatorFeatures(operatorPoint(view)).shapes.count, 1)
    }

    func testAFixTheCoreWillNotStandBehindIsNotDrawn() {
        XCTAssertNil(operatorPoint(JSON.parse(#"{"usable":false,"latitude":41.6983,"longitude":44.85148}"#)))
        XCTAssertNil(operatorPoint(JSON.parse(#"{"usable":true,"latitude":null,"longitude":null}"#)))
        XCTAssertNil(operatorPoint(nil))
        XCTAssertEqual(operatorFeatures(nil).shapes.count, 0)
    }
}
