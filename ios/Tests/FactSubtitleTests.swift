import XCTest
@testable import Aircast

private func fact(_ units: String, enums: [String] = [], bits: [String] = [], bool: Bool = false) -> Fact {
    Fact(
        path: "p", name: "N", description: "d", units: units, valueString: "0.15", value: .string("0.15"),
        enumStrings: enums, enumIndex: 0,
        bitmaskStrings: bits, bitmaskValues: bits.indices.map { Int64(1) << Int64($0) },
        isBool: bool, isString: false, readOnly: false
    )
}

final class FactSubtitleTests: XCTestCase {
    func testANumberKeepsTheUnitItIsMeasuredIn() {
        XCTAssertEqual(factSubtitle(fact("m")), "m")
    }

    func testARowShowingAWordDropsTheUnitBecauseMediumIsNotMeasuredInSeconds() {
        XCTAssertEqual(factSubtitle(fact("s", enums: ["Soft", "Medium"])), "")
        XCTAssertEqual(factSubtitle(fact("s", bits: ["Alt", "Circle"])), "")
        XCTAssertEqual(factSubtitle(fact("s", bool: true)), "")
    }
}
