import XCTest
@testable import Aircast

final class RcToParamTests: XCTestCase {
    func testEveryFieldMustBeANumberBeforeTheMappingIsSent() {
        XCTAssertEqual(rcToParam("1.0", "6.5", 2, "0", "12"), RcToParam(scale: 1.0, center: 6.5, tuningIndex: 2, min: 0.0, max: 12.0))
        XCTAssertNil(rcToParam("1.0", "", 0, "0", "12"))
        XCTAssertNil(rcToParam("x", "6.5", 0, "0", "12"))
    }
}
