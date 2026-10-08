import XCTest
@testable import Aircast

final class VtolStateCellTests: XCTestCase {
    func testAVtolShowsItsFlightStateAndOffersTheOppositeTransition() throws {
        let forward = try XCTUnwrap(vtolState(JSON.parse(#"{"vtol":true,"vtolInFwdFlight":true,"flying":true}"#)))
        XCTAssertEqual(forward.label, "FW(vtol)")
        XCTAssertEqual(forward.transition, "vtolTransitionToMultiRotor")
        XCTAssertEqual(vtolState(JSON.parse(#"{"vtol":true,"vtolInFwdFlight":false}"#))?.label, "MR(vtol)")
        XCTAssertNil(vtolState(JSON.parse(#"{"vtol":false}"#)))
    }
}
