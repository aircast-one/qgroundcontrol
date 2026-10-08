import XCTest
@testable import Aircast

final class ApmFollowScreenTests: XCTestCase {
    func testAVehicleWithoutFollowParametersHasNoPageAndOffsetsReadWithOneDecimal() throws {
        XCTAssertNil(apmFollow(JSON.parse(#"{"available":false}"#)))
        let follow = try XCTUnwrap(apmFollow(JSON.parse(#"""
            {"available":true,"enabled":true,"waiting":false,"supported":true,"showSettings":true,"rover":false,
            "positionOptions":["Maintain Current Offsets","Specify Offsets"],"positionIndex":1,
            "pointOptions":["a","b","c"],"pointIndex":-1,"angle":45.0,"distance":5.0,"height":5.0}
            """#)))
        XCTAssertTrue(follow.showSettings)
        XCTAssertEqual(follow.pointIndex, -1)
        XCTAssertEqual(follow.positionOptions[follow.positionIndex], "Specify Offsets")
        XCTAssertEqual(oneDecimal(follow.angle), "45.0")
    }

    func testOffsetsShowAndTypeInTheAppDistanceUnits() throws {
        let follow = try XCTUnwrap(apmFollow(JSON.parse(#"""
            {"available":true,"distance":3.048,"height":6.096,"horizontalUnit":"ft","horizontalMetresPerUnit":0.3048,
            "verticalUnit":"ft","verticalMetresPerUnit":0.3048}
            """#)))
        XCTAssertEqual(follow.horizontal.text(follow.distance), "10.0 ft")
        XCTAssertEqual(follow.vertical.text(follow.height), "20.0 ft")
        XCTAssertEqual(apmFollow(JSON.parse(#"{"available":true,"distance":5.0}"#))?.horizontal.text(5.0), "5.0 m")
    }

    func testATapSetsTheHeadingOfTheVehicleAroundTheGroundStation() {
        XCTAssertEqual(headingOfTap(0, 10), 0.0, accuracy: 1e-9)
        XCTAssertEqual(headingOfTap(10, 0), 90.0, accuracy: 1e-9)
        XCTAssertEqual(headingOfTap(0, -10), 180.0, accuracy: 1e-9)
        XCTAssertEqual(headingOfTap(-10, 0), 270.0, accuracy: 1e-9)
    }
}
