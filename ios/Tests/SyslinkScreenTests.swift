import XCTest
@testable import Aircast

final class SyslinkScreenTests: XCTestCase {
    func testTheRadioSettingsReadAsSyslinkComponentShowsThem() throws {
        let radio = try XCTUnwrap(syslink(JSON.parse(#"{"available":true,"channel":80,"address":"e7e7e7e7e7","rate":2,"rates":["750Kb/s","1Mb/s","2Mb/s"],"channelHint":"Channel can be between 0 and 125","addressHint":"Address in hex. Default is E7E7E7E7E7."}"#)))
        XCTAssertEqual(radio.channel, 80)
        XCTAssertEqual(radio.rates[radio.rate], "2Mb/s")
        XCTAssertNil(syslink(JSON.parse(#"{"available":false}"#)))
    }

    func testTheAddressFieldTakesHexOnlyAsItsRegExpValidatorDoes() {
        XCTAssertTrue(hexAddress("E7e7"))
        XCTAssertTrue(hexAddress(""))
        XCTAssertFalse(hexAddress("E7G"))
        XCTAssertFalse(hexAddress("E7E7E7E7E7E"), "the address is 40 bits, ten hex digits")
    }
}
