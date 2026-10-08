import XCTest
@testable import Aircast

final class ApmAirframeScreenTests: XCTestCase {
    func testClassesTheirFilteredTypesAndTheInvalidFlagComeFromTheCore() throws {
        let read = try XCTUnwrap(apmAirframe(JSON.parse(#"""
            {"available":true,"help":"Currently set","frameClass":2,"frameType":3,"invalidText":"Invalid setting for FRAME_TYPE. Click to Reset.",
            "classes":[{"name":"Quad","value":1,"chosen":false,"image":"QuadRotorX.svg","types":[{"name":"X","value":1}],"valid":true},
                       {"name":"Hexa","value":2,"chosen":true,"image":"AirframeUnknown.svg","types":[{"name":"X","value":1},{"name":"Plus","value":0}],"valid":false}]}
            """#)))
        XCTAssertEqual(read.classes.map(\.name), ["Quad", "Hexa"])
        XCTAssertEqual(read.frameType, 3)
        XCTAssertFalse(read.classes[1].valid)
        XCTAssertEqual(read.classes[1].types.map(\.value), [1, 0])
        XCTAssertNil(apmAirframe(JSON.parse(#"{"available":false}"#)))
    }
}
