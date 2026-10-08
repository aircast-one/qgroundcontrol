import XCTest
@testable import Aircast

final class PlainNumberTests: XCTestCase {
    func testALimitInScientificNotationIsSpelledOut() {
        XCTAssertEqual(plainNumber("3e+05"), "300000")
        XCTAssertEqual(plainNumber("-1e3"), "-1000")
    }

    func testAWholeNumberLosesItsDecimalTail() {
        XCTAssertEqual(plainNumber("200"), "200")
        XCTAssertEqual(plainNumber("200.000"), "200")
    }

    func testAFractionalLimitKeepsItsDigits() {
        XCTAssertEqual(plainNumber("0.05"), "0.05")
        XCTAssertEqual(plainNumber("1.50"), "1.5")
    }

    func testALimitTooLargeToSpellOutIsLeftAlone() {
        XCTAssertEqual(plainNumber("3.4e38"), "3.4e38")
    }

    func testAnythingThatIsNotANumberPassesThroughUntouched() {
        XCTAssertEqual(plainNumber(""), "")
        XCTAssertEqual(plainNumber("Auto"), "Auto")
    }
}
