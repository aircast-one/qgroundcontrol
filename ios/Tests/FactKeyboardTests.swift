import XCTest
@testable import Aircast

final class FactKeyboardTests: XCTestCase {
    private func fact(_ min: String, isString: Bool = false, isBool: Bool = false, whole: Bool = false) -> Fact {
        Fact(
            path: "p", name: "n", description: "", units: "", valueString: "0",
            value: .number(0), enumStrings: [], enumIndex: -1,
            isBool: isBool, isString: isString, wholeNumbersOnly: whole, readOnly: false, minString: min
        )
    }

    func testAParameterThatCannotGoNegativeGetsTheNumberPad() {
        XCTAssertEqual(factKeyboard(fact("200")), .decimalPad)
        XCTAssertEqual(factKeyboard(fact("0")), .decimalPad)
    }

    func testAParameterThatCanGoNegativeKeepsAKeyboardWithAMinus() {
        XCTAssertEqual(factKeyboard(fact("-50")), .numbersAndPunctuation)
        XCTAssertEqual(factKeyboard(fact("-3.4e38")), .numbersAndPunctuation)
    }

    func testAnUnstatedMinimumKeepsAMinusTypableRatherThanMakingItUntypable() {
        XCTAssertEqual(factKeyboard(fact("")), .numbersAndPunctuation)
        XCTAssertEqual(factKeyboard(fact("not a number")), .numbersAndPunctuation)
    }

    func testTextAndBooleanFactsAreNeverGivenANumberPad() {
        XCTAssertEqual(factKeyboard(fact("0", isString: true)), .default)
        XCTAssertEqual(factKeyboard(fact("0", isBool: true)), .default)
    }

    func testATextValueIsReadableWithoutEnteringTheControlThatWritesIt() {
        XCTAssertEqual(
            factValueLines(fact("0", isString: true)),
            4,
            "a comma list overflows a single-line field - Flight Modes shows six rows of "
                + "Acro,Circle,Drift,Sport,Flip,Bra... - and reading the rest means tapping into an "
                + "editable field, which on a phone also raises the keyboard over the page"
        )
    }

    func testANumberKeepsOneLineBecauseANumberDoesNotOverflow() {
        XCTAssertEqual(factValueLines(fact("200")), 1)
        XCTAssertEqual(factValueLines(fact("-50")), 1)
        XCTAssertEqual(factValueLines(fact("0", isBool: true)), 1)
    }

    func testAWrappingFieldMustNotLetTheReturnKeyIntoASettingsValue() {
        XCTAssertEqual(
            typedValue("Acro,\nCircle"),
            "Acro,Circle",
            "the multi-line field is what puts Enter on the keyboard, so the newline is a hazard this change introduces rather than one it found"
        )
    }

    func testAFractionTypedIntoAWholeNumberSettingIsRefusedNotSilentlyTruncated() {
        XCTAssertEqual(
            truncationRefusal(fact("0", whole: true), "3.7"),
            "Invalid number",
            "FactTextField validates the typed text, and QString(\"3.7\").toInt fails, so FactMetaData::convertAndValidateCooked answers Invalid number"
        )
    }

    func testAWholeNumberInAWholeNumberSettingPasses() {
        XCTAssertNil(truncationRefusal(fact("0", whole: true), "4"))
        XCTAssertNil(truncationRefusal(fact("0", whole: true), " 12 "))
        XCTAssertNil(truncationRefusal(fact("-5", whole: true), "-3"))
    }

    func testARealTypedSettingStillTakesFractions() {
        XCTAssertNil(
            truncationRefusal(fact("0"), "3.7"),
            "decimalPlaces is a different question - a real fact declaring zero decimals is what you write for a percentage, "
                + "and refusing fractions there would reject values the vehicle accepts"
        )
    }

    func testTextThatIsNotANumberIsLeftToTheVehiclesOwnValidator() {
        XCTAssertNil(truncationRefusal(fact("0", whole: true), "not a number"))
        XCTAssertNil(truncationRefusal(fact("0", whole: true), ""))
        XCTAssertNil(
            truncationRefusal(fact("-5", whole: true), "NaN"),
            "NaN == floor(NaN) is false, so the fraction message would fire for something that is not a fraction"
        )
        XCTAssertNil(truncationRefusal(fact("-5", whole: true), "Infinity"))
    }

    func testAWholeNumberSettingThatCannotGoNegativeGetsAKeypadWithNoDecimalPoint() {
        XCTAssertEqual(factKeyboard(fact("0", whole: true)), .numberPad)
        XCTAssertEqual(factKeyboard(fact("-5", whole: true)), .numbersAndPunctuation)
        XCTAssertEqual(factKeyboard(fact("0")), .decimalPad)
    }
}
