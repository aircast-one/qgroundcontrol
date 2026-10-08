import XCTest
@testable import Aircast

private func fact(_ value: String, _ strings: [String], _ index: Int) -> Fact {
    Fact(
        path: "p", name: "n", description: "", units: "", valueString: value, value: .string(value),
        enumStrings: strings, enumIndex: index, isBool: false, isString: false, readOnly: true
    )
}

final class EnumLabelTests: XCTestCase {
    func testAnEnumReadsAsItsNameNotItsNumber() {
        XCTAssertEqual(enumLabel(fact("6", ["Stabilize", "Land", "RTL"], 2)), "RTL")
    }

    func testAPlainValueReadsAsItself() {
        XCTAssertEqual(enumLabel(fact("42", [], -1)), "42")
    }

    func testAnIndexOutsideTheListFallsBackToTheValue() {
        XCTAssertEqual(enumLabel(fact("99", ["A", "B"], 7)), "99")
        XCTAssertEqual(enumLabel(fact("99", ["A", "B"], -1)), "99")
    }

    func testASettingsChoiceReadsInSentenceCaseButAnUnlistedValueStaysAsTyped() {
        XCTAssertEqual(shownEnumLabel(fact("0", ["Video Stream Disabled", "UDP h.264 Video Stream"], 0)), "Video stream disabled")
        XCTAssertEqual(shownEnumLabel(fact("1", ["Fit Width", "Fit Height"], 1)), "Fit height")
        XCTAssertEqual(shownEnumLabel(fact("My Value", [], -1)), "My Value")
    }
}
