import XCTest
@testable import Aircast

final class GuidedStepperTests: XCTestCase {
    func testTheHoldNamesItsTargetAndSaysAnywayOnlyForANonBlockingWarning() {
        XCTAssertEqual(guidedCommitLabel("Take off \u{00B7} 3.0 m", nil), "Take off \u{00B7} 3.0 m")
        XCTAssertEqual(guidedCommitLabel("Take off \u{00B7} 3.0 m", Readiness(text: "GPS off", blocks: false)), "Take off anyway \u{00B7} 3.0 m")
        XCTAssertEqual(guidedCommitLabel("Take off \u{00B7} 3.0 m", Readiness(text: "GPS off", blocks: true)), "Take off \u{00B7} 3.0 m")
        XCTAssertEqual(holdLabel(guidedCommitLabel("Take off \u{00B7} 3.0 m", Readiness(text: "GPS off", blocks: false))), "Hold to take off anyway \u{00B7} 3.0 m")
    }

    func testTakeoffPresetsAreTheCommonHeightsTheVehicleAllows() {
        XCTAssertEqual(guidedPresets("m", 3.0, 121.9), [5.0, 10.0, 20.0, 50.0])
        XCTAssertEqual(guidedPresets("m", 3.0, 15.0), [5.0, 10.0])
        XCTAssertEqual(guidedPresets("ft", 10.0, 400.0), [15.0, 30.0, 60.0, 150.0])
    }

    func testQuickPicksClimbTenAndTwentyFromTheTargetAndStopAtTheMaximum() {
        let picks = guidedQuickPicks(42.0, 1.0, 120.0, "m")
        XCTAssertEqual(picks.map(\.0), ["+10 m", "+20 m", "Max 120 m"])
        XCTAssertEqual(picks.map(\.1), [52.0, 62.0, 120.0])
        XCTAssertEqual(guidedQuickPicks(115.0, 1.0, 120.0, "m")[1].1, 120.0, accuracy: 1e-9)
    }

    func testAStepMovesOneUnitAndStaysInsideTheRangeLikeGuidedValueSliderStep() {
        XCTAssertEqual(guidedStepped(25.0, 1, 1.0, 120.0, "m"), 26.0, accuracy: 1e-9)
        XCTAssertEqual(guidedStepped(119.6, 1, 1.0, 120.0, "m"), 120.0, accuracy: 1e-9)
        XCTAssertEqual(guidedStepped(1.4, -1, 1.0, 120.0, "m"), 1.0, accuracy: 1e-9)
    }

    func testValuesRoundToTenthsInMetricAndWholeNumbersOtherwise() {
        XCTAssertEqual(guidedRounded(25.43, "m"), 25.4, accuracy: 1e-9)
        XCTAssertEqual(guidedRounded(25.43, "ft"), 25.0, accuracy: 1e-9)
        XCTAssertEqual(guidedValueText(25.4, "m"), "25.4")
        XCTAssertEqual(guidedValueText(25.0, "ft"), "25")
    }

    func testRoundingFollowsTheExactBinaryValueLikeBigDecimalHalfUp() {
        XCTAssertEqual(guidedRounded(0.15, "m"), 0.1, accuracy: 1e-9)
        XCTAssertEqual(guidedRounded(0.25, "m"), 0.3, accuracy: 1e-9)
        XCTAssertEqual(guidedRounded(1.05, "m"), 1.1, accuracy: 1e-9)
        XCTAssertEqual(guidedRounded(2.5, "ft"), 3.0, accuracy: 1e-9)
        XCTAssertEqual(guidedRounded(-0.15, "m"), -0.1, accuracy: 1e-9)
    }

    func testTheReadingRoundsHalfUpOnTheShortestDecimalLikeStringFormat() {
        XCTAssertEqual(guidedValueText(0.15, "m"), "0.2")
        XCTAssertEqual(guidedValueText(0.25, "m"), "0.3")
        XCTAssertEqual(guidedValueText(2.5, "ft"), "3")
    }

    func testATypedValueIsClampedAndRoundedAndGarbageIsIgnored() {
        XCTAssertEqual(guidedTyped("500", 1.0, 120.0, "m")!, 120.0, accuracy: 1e-9)
        XCTAssertEqual(guidedTyped("12,46", 1.0, 120.0, "m")!, 12.5, accuracy: 1e-9)
        XCTAssertNil(guidedTyped("x", 1.0, 120.0, "m"))
        XCTAssertNil(guidedBounds(nil, 10.0))
    }

    func testTheReadingCarriesItsUnitOnOneLine() {
        XCTAssertEqual(guidedReading(10.0, "ft"), "10 ft")
        XCTAssertEqual(guidedReading(12.5, "m"), "12.5 m")
        XCTAssertEqual(guidedReading(3.0, ""), "3")
    }

    func testThePanelNamesTheVehicleOnlyWhenThereIsAChoiceOfVehicles() {
        XCTAssertNil(guidedVehicle(1, "Quadrotor 128"))
        XCTAssertEqual(guidedVehicle(2, "Quadrotor 128"), "Quadrotor 128")
        XCTAssertNil(guidedVehicle(2, nil))
    }
}
