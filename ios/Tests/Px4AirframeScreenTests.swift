import XCTest
@testable import Aircast

final class Px4AirframeScreenTests: XCTestCase {
    func testTheCurrentAirframePreselectsItsGroupAndTheApplyTextLosesItsHtmlBreaks() throws {
        let read = try XCTUnwrap(px4Airframes(JSON.parse(
            #"{"available":true,"autostartId":4001,"custom":false,"heading":"You've connected a Generic Quadcopter.","currentType":"Quadrotor x","#
                + #""currentIndex":0,"applyTitle":"Apply and Restart","applyText":"a<br><br>b","types":[{"name":"Quadrotor x","airframes":[{"name":"Generic Quadcopter","autostartId":4001}]}]}"#
        )))
        XCTAssertEqual(initialSelection(read), AirframeSelection(group: "Quadrotor x", index: 0))
        XCTAssertEqual(read.applyText, "a\n\nb")
        XCTAssertEqual(read.groups.first?.airframes.first?.autostartId, 4001)
        XCTAssertNil(px4Airframes(JSON.parse(#"{"available":false}"#)))
    }

    func testEachAirframeGroupShowsItsQgcPicture() {
        XCTAssertEqual(airframeImageAsset("QuadRotorX"), "Airframe/QuadRotorX")
        XCTAssertEqual(airframeImageAsset("QuadRotorX.svg"), "Airframe/QuadRotorX")
        XCTAssertNil(airframeImageAsset(""))
    }
}
