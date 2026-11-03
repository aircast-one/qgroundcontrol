import XCTest
@testable import Aircast

final class LensTests: XCTestCase {
    func testTheLensViewReadsTheCoresSummaryAndMeasuredZooms() {
        let lens = lensReading(JSON.parse(#"{"class":"Lens","summary":"Field of view 62°","approximate":true,"measuring":false,"progress":"","error":"","canMeasure":true,"reason":"","samples":[{"zoom":50.0},{"zoom":0.0}]}"#))!
        XCTAssertEqual(lens.summary, "Field of view 62°")
        XCTAssertTrue(lens.approximate)
        XCTAssertNil(lens.error)
        XCTAssertNil(lens.reason)
        XCTAssertEqual(lens.measuredZooms, [50, 0])
        XCTAssertNil(lensReading(JSON.parse(#"{"class":"Orbit"}"#)))
    }

    func testMeasuredZoomsAreListedInOrder() {
        XCTAssertEqual(measuredText([50, 0]), "Measured at 0%, 50% zoom")
        XCTAssertNil(measuredText([]))
    }
}
