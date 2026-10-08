import XCTest
@testable import Aircast

private func fact(_ min: String = "", _ max: String = "", _ defaultValue: String = "", bounded: Bool = true, isEnum: Bool = false) -> Fact {
    Fact(
        path: "p",
        name: "RTL_ALT",
        description: "Return altitude",
        units: "m",
        valueString: "40",
        value: .number(40),
        enumStrings: isEnum ? ["Off", "On"] : [],
        enumIndex: -1,
        isBool: false,
        isString: false,
        readOnly: false,
        minString: min,
        maxString: max,
        minIsDefaultForType: !bounded,
        maxIsDefaultForType: !bounded,
        defaultValueString: defaultValue
    )
}

private func with(_ fact: Fact, _ change: (inout Fact) -> Void) -> Fact {
    var changed = fact
    change(&changed)
    return changed
}

final class ParameterRangeLineTests: XCTestCase {
    func testABoundedParameterWithADefaultReadsLikeThePenpotEditSheet() {
        XCTAssertEqual(parameterRangeLine(fact("2", "300", "15")), "Range 2–300 m · default 15 m")
    }

    func testTypeLimitsAndAbsentDefaultsSayNothing() {
        XCTAssertEqual(parameterRangeLine(fact("-3.4e38", "3.4e38", bounded: false)), "")
    }

    func testAnEnumShowsItsRangeAndDefaultWithoutUnitsAsParameterEditorDialogDoes() {
        XCTAssertEqual(parameterRangeLine(fact("0", "1", "0", isEnum: true)), "Range 0–1 · default 0")
    }

    func testALoneBoundIsShownOnItsOwn() {
        XCTAssertEqual(parameterRangeLine(with(fact("2", "")) { $0.maxIsDefaultForType = true }), "Min 2 m")
        XCTAssertEqual(parameterRangeLine(with(fact("", "300")) { $0.minIsDefaultForType = true }), "Max 300 m")
    }

    func testRebootNotesFollowTheFactLikeParameterEditorDialog() {
        XCTAssertEqual(parameterRebootNotes(with(fact()) { $0.vehicleRebootRequired = true }), ["Vehicle reboot required after change"])
        XCTAssertEqual(parameterRebootNotes(fact()), [])
    }
}
