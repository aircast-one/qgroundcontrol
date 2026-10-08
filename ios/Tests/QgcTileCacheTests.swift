import XCTest
@testable import Aircast

final class QgcTileCacheTests: XCTestCase {
    private let bingPrefix = "0308415137"

    func testAHashMatchesTheKeyQGroundControlWrote() {
        XCTAssertEqual(tileHash(bingPrefix, 2, 0, 2), "03084151370000000000000002002")
        XCTAssertEqual(tileHash(bingPrefix, 2, 3, 0), "03084151370000000300000000002")
        XCTAssertEqual(tileHash(bingPrefix, 2, 1, 1), "03084151370000000100000001002")
    }

    func testAHashIsAlwaysTheFull29Characters() {
        XCTAssertEqual(tileHash(bingPrefix, 0, 0, 0).count, 29)
        XCTAssertEqual(tileHash(bingPrefix, 20, 1048575, 1048575).count, 29)
    }

    func testEachFieldKeepsItsOwnWidth() {
        let hash = Array(tileHash(bingPrefix, 17, 119846, 79314))
        XCTAssertEqual(String(hash[0..<10]), bingPrefix)
        XCTAssertEqual(String(hash[10..<18]), "00119846")
        XCTAssertEqual(String(hash[18..<26]), "00079314")
        XCTAssertEqual(String(hash[26..<29]), "017")
    }
}
