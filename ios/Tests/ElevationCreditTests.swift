import XCTest
@testable import Aircast

final class ElevationCreditTests: XCTestCase {
    func testTheTerrainProfileCreditsItsElevationProviderLikePlanView() {
        XCTAssertEqual("Powered by Copernicus", elevationCredit("Copernicus"))
        XCTAssertNil(elevationCredit(""))
    }
}
