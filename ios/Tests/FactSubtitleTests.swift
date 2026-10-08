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

final class RunningTitleTests: XCTestCase {
    func testANamedRoutineReadsAsItself() {
        XCTAssertEqual(runningTitle("Compass"), "Calibrating Compass")
    }

    func testACalibrationTheScreenDidNotStartStillSaysWhatIsHappening() {
        XCTAssertEqual(runningTitle(""), "Calibration in progress")
        XCTAssertEqual(runningTitle("  "), "Calibration in progress")
    }
}

final class OperatorDistanceTests: XCTestCase {
    func testTheServedTextIsShownAsItsOwnReadingSplitIntoNumberAndUnitLikeEveryOtherReading() {
        let view = JSON.parse(#"{"distanceToVehicleText":"316.8 m"}"#)

        XCTAssertEqual(operatorDistance(view), [Instrument(label: "From you", reading: "316.8 m", value: "316.8", units: "m")])
        XCTAssertEqual(operatorDistance(JSON.parse(#"{"distanceToVehicleText":"316.8"}"#)), [Instrument(label: "From you", reading: "316.8", value: "316.8", units: "")])
    }

    func testNothingIsShownWhenTheCoreWithholdsTheDistance() {
        XCTAssertEqual(operatorDistance(JSON.parse(#"{"distanceToVehicleText":null}"#)), [])
        XCTAssertEqual(operatorDistance(JSON.parse("{}")), [])
        XCTAssertEqual(operatorDistance(nil), [])
    }
}

final class RowWidthTests: XCTestCase {
    func testFourOrFewerReadingsStayOnOneLine() {
        XCTAssertEqual(rowWidth(4), 4)
        XCTAssertEqual(rowWidth(3), 3)
    }

    func testMoreThanFourAreSplitEvenlyRatherThanLeavingAnOrphan() {
        XCTAssertEqual(rowWidth(5), 3)
        XCTAssertEqual(rowWidth(6), 3)
        XCTAssertEqual(rowWidth(7), 4)
    }
}
