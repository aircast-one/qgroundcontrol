import XCTest
@testable import Aircast

final class JSONTests: XCTestCase {
    func testSwiftNumbersAndBoolsBridgeToTheirJsonKinds() {
        XCTAssertEqual(.bool(true), JSON(true))
        XCTAssertEqual(.bool(false), JSON(false as Any))
        XCTAssertEqual(.number(5), JSON(5))
        XCTAssertEqual(.number(5), JSON(Int64(5)))
        XCTAssertEqual(.number(2.5), JSON(2.5))
        XCTAssertEqual(.array([.number(1), .bool(true), .string("a"), .null]), JSON([1, true, "a", NSNull()] as [Any]))
        XCTAssertEqual(.string("x"), JSON(JSON.string("x")))
    }

    func testBoolReadsTextCaseInsensitivelyAndNumbersByZero() {
        XCTAssertEqual(true, JSON.string("TRUE").boolOrNil)
        XCTAssertEqual(false, JSON.string("False").boolOrNil)
        XCTAssertNil(JSON.string("1").boolOrNil)
        XCTAssertEqual(true, JSON.number(2).boolOrNil)
        XCTAssertEqual(false, JSON.number(0).boolOrNil)
        XCTAssertTrue(JSON.null.bool(true))
    }

    func testTruthyAcceptsNumericTextButNotZero() {
        XCTAssertTrue(JSON.string("1").truthy)
        XCTAssertTrue(JSON.string("True").truthy)
        XCTAssertFalse(JSON.string("0.0").truthy)
        XCTAssertFalse(JSON.string("no").truthy)
        XCTAssertFalse(JSON.null.truthy)
    }

    func testWholeNumbersPrintWithoutAFractionOnlyBelowTheExactRange() {
        XCTAssertEqual("42", JSON.number(42).string)
        XCTAssertEqual("999999999999999", JSON.number(999_999_999_999_999).string)
        XCTAssertEqual("1000000000000000.0", JSON.number(1e15).string)
        XCTAssertEqual("1.5", JSON.number(1.5).string)
    }

    func testWholeNumbersBecomeIntegersBelowNineQuadrillionAndStayDoublesAbove() {
        XCTAssertTrue(JSON.number(8e15).any is Int64)
        XCTAssertTrue(JSON.number(9.5e15).any is Double)
        XCTAssertTrue(JSON.number(1.5).any is Double)
        XCTAssertEqual("8000000000000000", JSON.encode(8e15))
        XCTAssertEqual("9500000000000000", JSON.encode(9.5e15))
        XCTAssertEqual("[3,null]", JSON.encode([3, NSNull()] as [Any]))
        XCTAssertEqual("null", JSON.encode(Double.nan))
    }
}
