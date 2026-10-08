import XCTest
@testable import Aircast

final class ApmSubFrameScreenTests: XCTestCase {
    func testFramesTheSelectionAndTheConfirmRuleComeFromTheCore() throws {
        let read = try XCTUnwrap(subFrames(JSON.parse(#"""
            {"available":true,"selected":1,"confirmFirst":true,"loadingDefaults":false,"loadError":"",
            "frames":[{"name":"BlueROV1","value":0,"hasDefaults":false},{"name":"BlueROV2/Vectored","value":1,"hasDefaults":true}]}
            """#)))
        XCTAssertEqual(read.frames.map(\.name), ["BlueROV1", "BlueROV2/Vectored"])
        XCTAssertEqual(read.selected, 1)
        XCTAssertTrue(read.confirmFirst)
        XCTAssertTrue(read.frames[1].hasDefaults)
        XCTAssertNil(subFrames(JSON.parse(#"{"available":false}"#)))
    }
}
