import XCTest
@testable import Aircast

final class FirmwareFenceTests: XCTestCase {
    private func view(_ body: String) -> JSON { JSON.parse(#"{"firmwareFence":\#(body)}"#) }

    func testAServedFenceCarriesItsRadiusItsTextAndItsCentre() throws {
        let fence = try XCTUnwrap(firmwareFence(view(#"{"radiusMetres":300.0,"radiusText":"300 m","centre":{"latitude":41.7,"longitude":44.8}}"#)))
        XCTAssertEqual(300.0, fence.radiusMetres, accuracy: 0.001)
        XCTAssertEqual("300 m", fence.radiusText)
        XCTAssertEqual(41.7, try XCTUnwrap(fence.centre).latitude, accuracy: 0.001)
    }

    func testNoFenceAtAllIsNotAFenceOfNoSize() {
        XCTAssertNil(firmwareFence(JSON.parse("{}")))
        XCTAssertNil(firmwareFence(nil))
    }

    func testARadiusOfZeroIsAbsentParametersNotACircleRoundHome() {
        XCTAssertNil(firmwareFence(view(#"{"radiusMetres":0.0,"radiusText":"0 m"}"#)))
    }

    func testAFenceWithNoHomeToSitOnStillReportsItsRadius() throws {
        let fence = try XCTUnwrap(firmwareFence(view(#"{"radiusMetres":300.0,"radiusText":"300 m","centre":null}"#)))
        XCTAssertNil(fence.centre)
        XCTAssertEqual("300 m", fence.radiusText)
    }

    func testTheFenceTheVehicleImposesCannotBeDrawnLikeOneThePlanCanEdit() {
        XCTAssertNotEqual(KEEP_IN_COLOUR, FIRMWARE_FENCE_COLOUR)
        XCTAssertNotEqual(KEEP_OUT_COLOUR, FIRMWARE_FENCE_COLOUR)
    }
}
