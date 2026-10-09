import XCTest
@testable import Aircast

final class QgcVersionTests: XCTestCase {
    func testVersionCarriesTheBuildAbiBitnessLikeQgcVersion() {
        XCTAssertEqual(qgcVersion("5.0.1", is64Bit: true), "5.0.1 64 bit")
        XCTAssertEqual(qgcVersion("5.0.1", is64Bit: false), "5.0.1 32 bit")
    }
}
