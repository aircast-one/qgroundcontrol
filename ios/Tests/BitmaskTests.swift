import XCTest
@testable import Aircast

final class BitmaskTests: XCTestCase {
    private func arming(_ value: Int64) -> Fact {
        Fact(
            path: "p", name: "ARMING_CHECK", description: "Arm Checks to Perform", units: "",
            valueString: String(value), value: .number(Double(value)),
            enumStrings: [], enumIndex: -1,
            bitmaskStrings: ["All", "Barometer", "Compass", "GPS lock"],
            bitmaskValues: [1, 2, 4, 8],
            isBool: false, isString: false, readOnly: false
        )
    }

    private func changed(_ fact: Fact, _ change: (inout Fact) -> Void) -> Fact {
        var copy = fact
        change(&copy)
        return copy
    }

    func testABitmaskIsNotAnEnumSoItDoesNotGetASingleChoicePicker() {
        let fact = arming(0)
        XCTAssertTrue(fact.isBitmask)
        XCTAssertFalse(fact.isEnum)
    }

    func testNoBitsSetSaysSoInsteadOfShowingZero() {
        XCTAssertEqual(bitmaskSummary(arming(0)), "None")
    }

    func testTheSetChecksAreNamedWhichIsTheWholePoint() {
        XCTAssertEqual(bitmaskSummary(arming(2 | 8)), "Barometer, GPS lock")
    }

    func testEveryBitSetReadsAsAllRatherThanAListOfEverything() {
        XCTAssertEqual(bitmaskSummary(arming(1 | 2 | 4 | 8)), "All")
    }

    func testAValueArrivingAsAStringIsStillRead() {
        let asText = changed(arming(6)) {
            $0.value = .null
            $0.valueString = "6.000"
        }
        XCTAssertEqual(bitmaskSummary(asText), "Barometer, Compass")
    }

    func testTogglingABitFlipsOnlyThatBit() {
        let raw = bitmaskRaw(arming(2 | 8))
        XCTAssertEqual(raw ^ 4, 2 | 4 | 8)
        XCTAssertEqual(raw ^ 2, 8)
    }

    func testAFactWhoseBitNamesAndValuesDisagreeIsNotTreatedAsABitmask() {
        XCTAssertFalse(changed(arming(0)) { $0.bitmaskValues = [1, 2] }.isBitmask)
    }

    func testCheckingAllClearsAndLocksTheRestLikeFactBitmaskFirstEntryIsAll() {
        let fact = changed(arming(2 | 8)) { $0.firstEntryIsAll = true }
        XCTAssertEqual(bitmaskToggled(fact, 2 | 8, 0), 1)
        XCTAssertFalse(bitmaskEntryEnabled(fact, 1, 2))
        XCTAssertTrue(bitmaskEntryEnabled(fact, 0, 2))
        XCTAssertEqual(bitmaskToggled(fact, 1, 0), 0, "unchecking All only clears its own bit")
        XCTAssertEqual(bitmaskToggled(arming(2), 2, 0), 2 | 1, "without the flag All is an ordinary bit")
    }
}
