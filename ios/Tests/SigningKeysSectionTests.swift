import XCTest
@testable import Aircast

final class SigningKeysSectionTests: XCTestCase {
    func testAKeyNeedsANameAndALongEnoughPassphraseOr64Hex() {
        XCTAssertFalse(canAddKey("", false, "longenough", 8))
        XCTAssertFalse(canAddKey("field", false, "short", 8))
        XCTAssertTrue(canAddKey("field", false, "longenough", 8))
        XCTAssertTrue(canAddKey("field", true, String(repeating: "ab", count: 32), 8))
        XCTAssertFalse(canAddKey("field", true, String(repeating: "ab", count: 31), 8))
        XCTAssertFalse(isHexKey(String(repeating: "zz", count: 32)))
    }

    func testTheViewListsKeysOnlyWhenTheCoreOwnsThem() throws {
        XCTAssertNil(signingKeys(JSON.parse(#"{"available":false}"#)))
        let keys = try XCTUnwrap(signingKeys(JSON.parse(#"{"available":true,"vehicle":true,"activeKey":"None","minPassphraseLength":8,"keys":[{"name":"field","inUse":false}]}"#)))
        XCTAssertEqual(keys.keys, [SigningKeyRow(name: "field", inUse: false)])
    }

    func testOnlyTheActiveKeyOffersDisableAndOthersSayAnotherKeyIsActive() throws {
        let keys = try XCTUnwrap(signingKeys(JSON.parse(#"{"available":true,"vehicle":true,"armed":false,"state":"on","linkName":"USB","activeKey":"field","keys":[{"name":"field","inUse":true,"activeOnVehicle":true},{"name":"bench","inUse":false,"activeOnVehicle":false}]}"#)))
        XCTAssertEqual(keyButtons(keys, keys.keys[0]), KeyButtons(enable: false, disable: true, otherActive: false, pending: false))
        XCTAssertEqual(keyButtons(keys, keys.keys[1]), KeyButtons(enable: false, disable: false, otherActive: true, pending: false))
        var off = keys
        off.activeKey = "None"
        off.state = "off"
        XCTAssertTrue(keyButtons(off, off.keys[1]).enable)
    }
}
