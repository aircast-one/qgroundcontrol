import XCTest
@testable import Aircast

final class PlanTransformTests: XCTestCase {
    func testAnOffsetSendsEastNorthUpAndTheTwoScopeFlags() {
        let metres = DistanceUnit(name: "m", metresPerUnit: 1)
        XCTAssertEqual([.number(10), .number(-5), .number(0), .bool(true), .bool(false)], offsetArgs("10", "-5", "", metres, metres, takeoff: true, landing: false))
        XCTAssertNil(offsetArgs("ten", "0", "0", metres, metres, takeoff: false, landing: false))
    }

    func testOffsetsTypedInFeetAreSentInMetresLikeTheCookedQgcFacts() {
        let view = JSON.parse(#"{"horizontalUnit":"ft","horizontalMetresPerUnit":0.3048,"verticalUnit":"m","verticalMetresPerUnit":1.0}"#)
        let feet = transformUnit(view, "horizontal")
        XCTAssertEqual(DistanceUnit(name: "ft", metresPerUnit: 0.3048), feet)
        XCTAssertEqual([.number(3.048), .number(0), .number(2), .bool(false), .bool(false)], offsetArgs("10", "0", "2", feet, transformUnit(view, "vertical"), takeoff: false, landing: false))
        XCTAssertEqual(DistanceUnit(name: "m", metresPerUnit: 1), transformUnit(nil, "horizontal"))
    }

    func testHomeComesFromTheTransformView() {
        XCTAssertEqual(TrackPoint(latitude: 47, longitude: 8), transformHome(JSON.parse(#"{"home":{"latitude":47.0,"longitude":8.0}}"#)))
        XCTAssertNil(transformHome(JSON.parse(#"{"home":null}"#)))
    }
}
