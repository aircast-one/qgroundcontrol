import XCTest
@testable import Aircast

final class MapScaleTests: XCTestCase {
    func testMetresAcrossTheBarFollowLatitudeAndZoom() throws {
        let equator = try XCTUnwrap(metresAcross(0.0, 16.0, 300.0))
        let south = try XCTUnwrap(metresAcross(-60.0, 16.0, 300.0))
        XCTAssertTrue(south < equator, "a degree of longitude is shorter away from the equator")
        XCTAssertEqual(equator, metresPerPixel(0.0, 16.0) * 300.0, accuracy: 1e-9)
    }

    func testABarThatCannotBeMeasuredIsNotDrawn() {
        XCTAssertNil(metresAcross(0.0, 16.0, 0.0))
        XCTAssertNil(metresAcross(.nan, 16.0, 200.0))
    }

    func testTheBarTakesTheCoresTextAndItsShareOfTheWidth() throws {
        let bar = try XCTUnwrap(mapScaleBar(JSON.parse(#"{"available":true,"text":"100 m","fraction":0.5}"#), 300.0))
        XCTAssertEqual(bar.text, "100 m")
        XCTAssertEqual(bar.pixels, 150.0, accuracy: 1e-9)
    }

    func testImperialTextIsPassedThroughBecauseTheCoreChoseTheUnits() throws {
        let bar = try XCTUnwrap(mapScaleBar(JSON.parse(#"{"available":true,"text":"500 ft","fraction":0.4}"#), 200.0))
        XCTAssertEqual(bar.text, "500 ft")
    }

    func testAScaleTheCoreCouldNotWorkOutDrawsNothing() {
        XCTAssertNil(mapScaleBar(JSON.parse(#"{"available":false}"#), 300.0))
        XCTAssertNil(mapScaleBar(JSON.parse(#"{"available":true,"text":"","fraction":0.5}"#), 300.0))
        XCTAssertNil(mapScaleBar(JSON.parse(#"{"available":true,"text":"100 m"}"#), 300.0))
        XCTAssertNil(mapScaleBar(nil, 300.0))
    }
}
