import XCTest
@testable import Aircast

final class OptTextTests: XCTestCase {
    func testAFieldTheCoreWithheldReadsAsAbsentNotAsTheWordNull() {
        let json = JSON.parse(#"{"altitudeText":null,"distanceText":"449 m"}"#)
        XCTAssertEqual(json["altitudeText"].string, "")
        XCTAssertEqual(json["distanceText"].string, "449 m")
    }

    func testAKeyThatWasNeverThereIsAbsentToo() {
        XCTAssertEqual(JSON.parse("{}")["nope"].string, "")
    }

    func testAWithheldElementOfAListReadsAsAbsent() {
        let list = JSON.parse(#"["a",null,"c"]"#)
        XCTAssertEqual(list[0].string, "a")
        XCTAssertEqual(list[1].string, "")
        XCTAssertEqual(list[2].string, "c")
    }
}
