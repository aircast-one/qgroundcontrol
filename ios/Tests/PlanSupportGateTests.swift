import XCTest
@testable import Aircast

final class PlanSupportGateTests: XCTestCase {
    private func plan(_ addFence: Bool, _ addRally: Bool) -> JSON {
        JSON.parse(#"{"actions":{"addFence":\#(addFence),"addRally":\#(addRally)}}"#)
    }

    func testAFirmwareWithoutFencesRefusesBothFenceShapesNotJustThePolygon() {
        let support = planSupport(plan(false, true))
        XCTAssertFalse(
            support.fence,
            "the Fence button adds an inclusion POLYGON and the Circle button an inclusion CIRCLE - both are geofences and both must answer to addFence. Circle carried no enabled gate at all and stayed live on a firmware that refuses fences"
        )
        XCTAssertTrue(support.rally, "rally is a separate capability and is unaffected")
    }

    func testAbsentActionsRefuseRatherThanAssume() {
        let nothing = planSupport(JSON.parse("{}"))
        XCTAssertFalse(nothing.fence)
        XCTAssertFalse(nothing.rally)
    }
}
