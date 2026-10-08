import XCTest
@testable import Aircast

final class RcSignalTests: XCTestCase {
    private func state(rcSupported: Bool = true, rcSignal: String = "72", rcSignalText: String = #""72%""#) -> FlyState? {
        flyState(JSON.parse(
            #"{"kind":"object","class":"FlyState","connected":true,"contactLost":false,"state":"disarmed","stateText":"Disarmed","staleNotice":"","mode":"Stabilize","rcSupported":\#(rcSupported),"rcSignal":\#(rcSignal),"rcSignalText":\#(rcSignalText)}"#
        ))
    }

    func testTheSentinelTheFirmwareSendsForUnknownReachesThisHeadAsNoTextAtAll() {
        XCTAssertNil(rcCell(state(rcSignal: "null", rcSignalText: "null")))
    }

    func testZeroIsAReadingTheVehicleChoseToSendAndSaysTheLinkIsDead() throws {
        let cell = try XCTUnwrap(rcCell(state(rcSignal: "0", rcSignalText: #""No signal""#)))
        XCTAssertEqual(cell.text, "No signal RC")
        XCTAssertEqual(cell.lost, true)
    }

    func testARealReadingIsTheCoresSentenceNotAPercentageThisHeadFormats() throws {
        let cell = try XCTUnwrap(rcCell(state()))
        XCTAssertEqual(cell.text, "72% RC")
        XCTAssertEqual(cell.lost, false)
    }

    func testAVehicleWithNoRadioNeverShowsTheCell() {
        XCTAssertNil(rcCell(state(rcSupported: false)))
    }

    func testNoVehicleIsNoCell() {
        XCTAssertNil(rcCell(nil))
    }
}

final class BatteryTextTests: XCTestCase {
    func testTheLevelNameMapsToTheLadderThisStripAlreadyDrew() {
        XCTAssertEqual(batteryLevelOf("normal"), .Normal)
        XCTAssertEqual(batteryLevelOf("caution"), .Caution)
        XCTAssertEqual(batteryLevelOf("warning"), .Warning)
        XCTAssertEqual(batteryLevelOf("critical"), .Critical)
    }

    func testAnUnknownOrAbsentLevelIsNotTreatedAsAnAlarm() {
        XCTAssertEqual(batteryLevelOf(nil), .Normal)
        XCTAssertEqual(batteryLevelOf("a level added later"), .Normal)
    }

    func testEachPackIsACellWithItsLabelAndTheLinesTheCoreChose() {
        let readings = batteryReadings(JSON.parse(#"""
            {"available":true,"level":"warning","indicatorPacks":[
                {"level":"caution","indicatorLabel":"B1","indicatorLines":["70%","11.10V"]},
                {"level":"warning","indicatorLabel":"B2","indicatorLines":["40%"]}]}
            """#))
        XCTAssertEqual(readings.map(\.text), ["B1 70% · 11.10V", "B2 40%"])
        XCTAssertEqual(readings.map(\.level), [.Caution, .Warning])
    }

    func testCombinedPacksReadAsOneRingWithTheCountBesideTheLimitingCharge() throws {
        let readings = batteryReadings(JSON.parse(#"{"available":true,"indicatorPacks":[{"level":"normal","indicatorLabel":null,"packCount":2,"indicatorLines":["57%"]}]}"#))
        XCTAssertEqual(readings.count, 1)
        let reading = try XCTUnwrap(readings.first)
        XCTAssertEqual(reading.text, "57%")
        XCTAssertEqual(batteryPercent(reading.text), 57)
        XCTAssertEqual(packCountText(reading.packs), "\u{00d7}2")
        XCTAssertEqual(packCountText(1), "")
    }

    func testASinglePackCarriesNoLabel() {
        XCTAssertEqual(
            batteryReadings(JSON.parse(#"{"available":true,"indicatorPacks":[{"level":"normal","indicatorLabel":null,"indicatorLines":["90%"]}]}"#)).map(\.text),
            ["90%"]
        )
    }

    func testNoBatteryIsNoCellRatherThanAnEmptyOne() {
        XCTAssertEqual(batteryReadings(nil), [])
        XCTAssertEqual(batteryReadings(JSON.parse(#"{"available":false}"#)), [])
        XCTAssertEqual(batteryReadings(JSON.parse(#"{"available":true,"indicatorPacks":[{"indicatorLines":[""]}]}"#)), [])
    }
}

final class GpsFixTests: XCTestCase {
    func testAFixTypeBelow2DIsNoFixAtAll() {
        XCTAssertEqual(fixLevel(0.0), FixLevel.None)
        XCTAssertEqual(fixLevel(1.0), FixLevel.None)
    }

    func test2DIsToldApartFrom3DRatherThanBothPassingAsAFix() {
        XCTAssertEqual(fixLevel(2.0), .TwoD)
        XCTAssertEqual(fixLevel(3.0), .Good)
        XCTAssertEqual(fixLevel(6.0), .Good)
    }

    func testAnUnreadableLockShowsNothingRatherThanClaimingAFix() {
        XCTAssertNil(fixLevel(.nan))
    }

    func testASatelliteCountIsNotShownAsReassuranceWhenThereIsNoFix() {
        XCTAssertEqual(satsText(FixLevel.None, "11"), "No fix")
    }

    func testA2DFixSaysSoNextToTheCount() {
        XCTAssertEqual(satsText(.TwoD, "11"), "11 sats · 2D only")
    }

    func testAGoodFixIsJustTheCount() {
        XCTAssertEqual(satsText(.Good, "11"), "11 sats")
    }

    func testTheRenderedLockTextIsNotAFixLevelSoWiringTheStringBackInHidesTheCell() {
        XCTAssertNil(fixLevel(Double("3D Lock") ?? .nan))
        XCTAssertNil(fixLevel(Double("None") ?? .nan))
    }
}

final class SatelliteCellTests: XCTestCase {
    func testASentenceDoesNotGetTheNounAppendedToIt() {
        XCTAssertEqual(satsText(FixLevel.None, "11"), "No fix")
        XCTAssertEqual(satsText(FixLevel.None, ""), "No fix")
    }

    func testACountCarriesTheNounOnce() {
        XCTAssertEqual(satsText(.Good, "11"), "11 sats")
        XCTAssertEqual(satsText(.TwoD, "11"), "11 sats · 2D only")
    }

    func testAFixWithNoCountReportedReadsAsGpsIndicatorsDashesRatherThanABareNoun() {
        XCTAssertEqual(satsText(.Good, ""), "--")
        XCTAssertEqual(satsText(.TwoD, ""), "2D only")
    }
}

final class IndicatorParameterWaitTests: XCTestCase {
    func testBatteryFailsafesWaitForParametersLikeToolIndicatorPageExpandedComponentWaitForParameters() {
        XCTAssertNil(indicatorParameterWait(JSON.parse(#"{"connected":true,"parametersReady":true,"parametersReason":"incomplete"}"#)))
        XCTAssertEqual(indicatorParameterWait(JSON.parse(#"{"connected":true,"parametersReady":false,"parametersReason":"loading"}"#)), "Waiting for parameters…")
        XCTAssertEqual(indicatorParameterWait(JSON.parse(#"{"connected":true,"parametersReady":false,"parametersReason":"unanswered"}"#)), "Waiting for parameters…")
        XCTAssertEqual(indicatorParameterWait(JSON.parse(#"{"connected":true,"parametersReady":false,"parametersReason":"skipped"}"#)), "Parameters not available")
    }
}

final class CompactStatusTests: XCTestCase {
    func testACompactCellKeepsOnlyTheReadingAndAnIconAloneWhenThereIsNone() {
        XCTAssertEqual(compactStatusText("B1 53%"), "53%")
        XCTAssertEqual(compactStatusText("12 sats"), "12")
        XCTAssertEqual(compactStatusText("12 sats · 2D only"), "12")
        XCTAssertEqual(compactStatusText("-70 dBm"), "-70")
        XCTAssertEqual(compactStatusText("No signal RC"), "")
    }

    func testANarrowStatusPillKeepsOnlyTheBatterySoNothingIsCutInHalf() {
        XCTAssertEqual(shownStatusCells(true), 1)
        XCTAssertEqual(shownStatusCells(false), COMPACT_STATUS_CELLS)
    }
}
