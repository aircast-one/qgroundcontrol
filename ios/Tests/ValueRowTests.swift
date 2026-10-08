import XCTest
@testable import Aircast

private func number(
    _ value: String,
    _ longDescription: String = "",
    min: String = "",
    max: String = "",
    bounded: Bool = false,
    isString: Bool = false,
    name: String = "MPC_XY_VEL_MAX",
    default fallback: String = "",
    units: String = "m"
) -> Fact {
    Fact(
        path: "vehicle.parameterManager.getParameter(-1,GF_MAX_VER_DIST)",
        name: name,
        description: "Max vertical distance from Home",
        units: units,
        valueString: value,
        value: Double(value).map(JSON.number) ?? .null,
        enumStrings: [],
        enumIndex: -1,
        isBool: false,
        isString: isString,
        readOnly: false,
        minString: min,
        maxString: max,
        minIsDefaultForType: !bounded,
        maxIsDefaultForType: !bounded,
        defaultValueString: fallback,
        longDescription: longDescription
    )
}

final class ValueRowTests: XCTestCase {
    func testALimitThatZeroDisablesReadsOffAtZeroAndItsValueOtherwise() {
        XCTAssertEqual(valueText(number("0", "Disabled if 0.")), VALUE_OFF)
        XCTAssertEqual(valueText(number("120", "Disabled if 0.")), "120 m")
        XCTAssertEqual(valueText(number("0", "Height above home.")), "0 m")
    }

    func testStepsFollowTheValuesOwnPrecisionAndStayInsideItsRange() {
        XCTAssertEqual(valueStep(number("12.0")), 0.1, accuracy: 1e-9)
        XCTAssertEqual(valueStep(number("30")), 1.0, accuracy: 1e-9)
        XCTAssertEqual(steppedValue(number("20", min: "0", max: "20", bounded: true), 1)!, 20.0, accuracy: 1e-9)
        XCTAssertEqual(steppedValue(number("30"), -1)!, 29.0, accuracy: 1e-9)
        XCTAssertEqual(steppedValue(number("0.7"), 1)!, 0.8, accuracy: 1e-9)
        XCTAssertEqual(roundedValue(number("30"), 3456.237), 3456.0, accuracy: 1e-9)
    }

    func testASliderOnlyAppearsForABoundedSensibleRange() {
        XCTAssertEqual(derivedSlider(number("0", min: "0", max: "10000", bounded: true))?.to, 10000)
        XCTAssertNil(derivedSlider(number("30")))
        XCTAssertNil(derivedSlider(number("30", min: "0", max: "340282346638528859811704183484516925440", bounded: true)))
    }

    func testNumbersOpenAsAValueRowAndTextStaysAField() {
        XCTAssertTrue(opensAsValue(number("30")))
        XCTAssertFalse(opensAsValue(number("127.0.0.1", isString: true)))
    }

    func testTurningALimitOnLandsOnASafeValueNeverOneStepAboveZero() {
        XCTAssertEqual(turnOnValue(number("0", "Disabled if 0.", name: "GF_MAX_VER_DIST", default: "0")), 120.0, accuracy: 1e-9)
        XCTAssertEqual(turnOnValue(number("0", "Disabled if 0.", name: "GF_MAX_HOR_DIST")), 500.0, accuracy: 1e-9)
        XCTAssertEqual(turnOnValue(number("0", "Disabled if 0.", name: "FENCE_ALT_MAX", units: "cm")), 12000.0, accuracy: 1e-9)
        XCTAssertEqual(turnOnValue(number("0", "Disabled if 0.", name: "OTHER", default: "80")), 80.0, accuracy: 1e-9)
        XCTAssertEqual(nudged(number("0", "Disabled if 0.", name: "GF_MAX_VER_DIST"), 0.0, 1, 0), 120.0, accuracy: 1e-9)
    }

    func testAltitudesAndDistancesStepInWholeMetresWhateverTheirPrecision() {
        XCTAssertEqual(valueStep(number("30.0", name: "RTL_RETURN_ALT")), 1.0, accuracy: 1e-9)
        XCTAssertEqual(valueStep(number("3000", name: "RTL_ALT", units: "cm")), 100.0, accuracy: 1e-9)
    }

    func testHoldingAStepButtonSpeedsUpAfterAWhile() {
        XCTAssertEqual(nudged(number("30"), 30.0, 1, 3), 31.0, accuracy: 1e-9)
        XCTAssertEqual(nudged(number("30"), 30.0, 1, 12), 40.0, accuracy: 1e-9)
    }

    func testReturnAndFenceSlidersCoverTheUsefulBandNotTheWholeTypeRange() {
        let slider = sliderFor(number("30", name: "RTL_RETURN_ALT"))
        XCTAssertEqual(slider?.from, 20)
        XCTAssertEqual(slider?.to, 500)
        XCTAssertEqual(sliderFor(number("0", min: "0", max: "10000", bounded: true, name: "GF_MAX_HOR_DIST"))?.from, 50)
    }

    func testAPendingValueReadsLikeTheRow() {
        XCTAssertEqual(valueTextOf(number("120", "Disabled if 0."), 0.0), VALUE_OFF)
        XCTAssertEqual(valueTextOf(number("30.0"), 45.5), "45.5 m")
    }

    func testALongChoiceLabelIsCutAtItsFirstClauseInTheRow() {
        XCTAssertEqual(rowChoiceLabel("Return at critical level, land at emergency level"), "Return at critical level")
        XCTAssertEqual(rowChoiceLabel("Hold mode"), "Hold mode")
    }

    func testALimitsSliderEndsOnNoLimitLikeDjisMaxDistance() {
        let limit = number("0", "Disabled if 0.", name: "GF_MAX_HOR_DIST")
        let slider = inlineSlider(limit)!
        XCTAssertEqual(valueTextOf(limit, 0.0), VALUE_NO_LIMIT)
        XCTAssertEqual(sliderPosition(slider, 0.0), slider.end)
        XCTAssertEqual(valueAtPosition(limit, slider, slider.end), 0.0, accuracy: 1e-9)
        XCTAssertEqual(valueAtPosition(limit, slider, slider.top), 5000.0, accuracy: 1e-9)
        XCTAssertEqual(valueAtPosition(limit, slider, 1234.4), 1234.0, accuracy: 1e-9)
    }

    func testAPlainRangeHasNoNoLimitStop() {
        let speed = number("12.0", min: "0", max: "20", bounded: true)
        let slider = inlineSlider(speed)!
        XCTAssertNil(slider.noLimitAt)
        XCTAssertEqual(valueAtPosition(speed, slider, slider.end), 20.0, accuracy: 1e-9)
    }

    func testOnlyATouchOnTheKnobGrabsItSoScrollingPastCannotMoveALimit() {
        XCTAssertTrue(grabsThumb(105, 100, 24))
        XCTAssertFalse(grabsThumb(300, 100, 24))
        XCTAssertEqual(thumbFraction(50, 0...100), 0.5)
        XCTAssertEqual(thumbFraction(500, 0...100), 1)
    }

    func testMotorStoppingFailsafesSinkBelowTheSafeOnes() {
        let options = ["Do nothing", "Hover", "Stop motors", "Return home", "Disarm", "Land"]
        XCTAssertEqual(safeFirst(options), [0, 1, 3, 5, 2, 4])
        XCTAssertTrue(dangerousChoice("Terminate"))
        XCTAssertFalse(dangerousChoice("Return home"))
    }
}
