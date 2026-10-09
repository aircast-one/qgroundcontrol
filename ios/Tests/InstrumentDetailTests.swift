import XCTest
@testable import Aircast

final class InstrumentDetailTests: XCTestCase {
    private func row(_ label: String, _ value: String, _ severity: Int = SEVERITY_SECONDARY) -> String {
        #"{"label":"\#(label)","value":"\#(value)","severity":\#(severity)}"#
    }

    private func battery(_ packs: String..., headline: String = "null") -> JSON {
        JSON.parse(#"{"kind":"object","class":"Battery","available":true,"packs":[\#(packs.joined(separator: ","))],"headline":\#(headline)}"#)
    }

    private func pack(_ rows: String...) -> String { #"{"rows":[\#(rows.joined(separator: ","))]}"# }

    func testABatteryThatIsNotThereHasNoDetail() {
        XCTAssertEqual(batteryDetail(nil), [])
        XCTAssertEqual(batteryDetail(JSON.parse(#"{"kind":"object","available":false,"packs":[]}"#)), [])
        XCTAssertNil(batteryHeadline(nil))
    }

    func testTheCoresPopupRowsAreShownInItsOrderWithTheirColouring() {
        let rows = batteryDetail(battery(pack(row("Time left", "4:10", 1), row("Charge", "30%", 1), row("Voltage", "11.10 V"))))
        XCTAssertEqual(rows.map(\.label), ["Time left", "Charge", "Voltage"])
        XCTAssertEqual(rows.map(\.severity), [1, 1, SEVERITY_SECONDARY])
    }

    func testTwoPacksReadAsOneLineEachWithTheLimitingOneMarked() {
        let view = battery(
            pack(row("Charge", "90%"), row("Voltage", "12.60 V")),
            pack(row("Charge", "79%", 1), row("Voltage", "12.30 V")),
            headline: #"{"text":"79%","detail":"","severity":0,"index":1}"#
        )
        XCTAssertEqual(batteryDetail(view), [
            DetailRow(label: "Battery 1", value: "90% \u{00b7} 12.60 V", severity: 0),
            DetailRow(label: "Battery 2 \u{00b7} lowest", value: "79% \u{00b7} 12.30 V", severity: 1),
        ])
    }

    func testTheHeadlineIsTheLimitingPackTheCorePickedWithItsLevelAndFailsafeMargin() {
        let view = battery(pack(), headline: #"{"text":"Critical","detail":"1:30 left","severity":2,"level":"critical","margin":"Returns home at 7%","index":0}"#)
        XCTAssertEqual(batteryHeadline(view), BatteryHeadline(text: "Critical", detail: "1:30 left", severity: 2, level: .Critical, margin: "Returns home at 7%", index: 0))
    }

    func testSilenceReadsInSecondsThenMinutes() {
        XCTAssertEqual(silenceText(12), "No data from the aircraft for 12 s")
        XCTAssertEqual(silenceText(150), "No data from the aircraft for 2 min")
    }

    func testTheVehicleChipSaysTheSignalIsLostAndForHowLong() {
        XCTAssertEqual(signalLostTitle(12), "Signal lost \u{00b7} 12 s")
        XCTAssertEqual(osdModeText(signalLostTitle(12)), "Signal lost")
        XCTAssertEqual(osdStatusNote(signalLostTitle(12)), "12 s")
        XCTAssertEqual(signalLostTitle(nil), "Signal lost")
    }

    func testTheChipCountsDownToTheLostLinkFailsafeThenNamesIt() {
        let home = LossFailsafe(action: "Return home", after: 10.0)
        XCTAssertEqual(signalLostTitle(7, failsafe: home), "Signal lost \u{00b7} 7 s \u{00b7} Return home in 3 s")
        XCTAssertEqual(signalLostTitle(12, failsafe: home), "Signal lost \u{00b7} 12 s \u{00b7} Return home")
        XCTAssertEqual(signalLostTitle(7, failsafe: LossFailsafe(action: "No failsafe", after: nil)), "Signal lost \u{00b7} 7 s \u{00b7} No failsafe")
        XCTAssertEqual(signalLostTitle(2, failsafe: home, compact: true), "Lost 2 s \u{00b7} Return home in 8 s", "the portrait chip keeps the countdown and shortens the rest")
    }

    func testTheCoresSpokenLinesAreReadInOrderAfterTheLastOneHeard() {
        let batch = speechBatch(JSON.parse(#"{"class":"Speech","last":4,"lines":[{"sequence":3,"text":"communication lost","volume":0.5},{"sequence":4,"text":"armed","volume":1}]}"#))
        XCTAssertEqual(batch, SpeechBatch(last: 4, lines: [SpokenLine(sequence: 3, text: "communication lost", volume: 0.5), SpokenLine(sequence: 4, text: "armed", volume: 1)]))
        XCTAssertEqual(speechPath(4), "view.speech(4)")
        XCTAssertNil(speechBatch(nil))
    }

    func testEveryPlaceholderQgcPrintsForAnUncomputedFactIsTreatedAsOne() {
        XCTAssertTrue(notYetComputed("--.--"))
        XCTAssertTrue(notYetComputed("--:--:--"))
        XCTAssertTrue(notYetComputed("--"))
        XCTAssertTrue(notYetComputed(" "))
        XCTAssertFalse(notYetComputed("0"))
        XCTAssertFalse(notYetComputed("11.10"))
    }

    func testADilutionTheReceiverNeverComputedIsNotAPrecisionClaim() {
        XCTAssertFalse(usableDop("--.--"))
        XCTAssertFalse(usableDop(""))
        XCTAssertFalse(usableDop("0"))
        XCTAssertFalse(usableDop("99999"))
        XCTAssertTrue(usableDop("1.4"))
    }

    func testGpsDetailIsTheLockPlusTheRowsTheCoreServed() throws {
        let view = JSON.parse(#"{"kind":"object","available":true,"satellites":11,"lock":3,"lockText":"3D Lock","rows":[{"label":"Satellites","value":"11"},{"label":"HDOP","value":"1.4"},{"label":"","value":"x"}]}"#)
        let gps = try XCTUnwrap(gpsStatus(view))
        XCTAssertEqual(gps.satellites, 11)
        XCTAssertEqual(fixLevel(gps.lock), .Good)
        XCTAssertEqual(gpsDetail(gps).map(\.label), ["GPS Lock", "Satellites", "HDOP"])
        XCTAssertEqual(gpsDetail(gps).first?.value, "3D Lock")
    }

    func testNoVehicleAndNoLockReadAsNothingRatherThanAFix() throws {
        XCTAssertNil(gpsStatus(JSON.parse(#"{"kind":"object","available":false,"rows":[]}"#)))
        let unlocked = try XCTUnwrap(gpsStatus(JSON.parse(#"{"kind":"object","available":true,"satellites":null,"lock":null,"rows":[]}"#)))
        XCTAssertNil(fixLevel(unlocked.lock))
        XCTAssertNil(unlocked.satellites)
        XCTAssertEqual(gpsDetail(unlocked), [])
    }

    func testTheLockRowShowsTheCoresLockTextSoRtkStatesReadAsGpsIndicatorPageSpellsThem() {
        let rtk = gpsStatus(JSON.parse(#"{"kind":"object","available":true,"satellites":20,"lock":6,"lockText":"RTK Fixed","rows":[]}"#))
        XCTAssertEqual(gpsDetail(rtk), [DetailRow(label: "GPS Lock", value: "RTK Fixed")])
        XCTAssertEqual(gpsDetail(nil).map(\.label), [])
    }

    func testTheCarryingLinkIsNamedAsTheOneInUse() {
        let links = VehicleLinks(available: true, links: [VehicleLink(commLost: false), VehicleLink(commLost: true)])
        let rows = linkDetail(links, ["Telemetry", "WiFi"], "Telemetry")
        XCTAssertEqual(rows.map(\.label), ["Telemetry", "WiFi"])
        XCTAssertEqual(rows.map(\.value), ["carrying", "no contact"])
    }

    func testALinkThatIsNeitherPrimaryNorLostIsStandingByNotBlank() {
        let links = VehicleLinks(available: true, links: [VehicleLink(commLost: false), VehicleLink(commLost: false)])
        XCTAssertEqual(linkDetail(links, ["Telemetry", "WiFi"], "Telemetry").map(\.value), ["carrying", "standing by"])
    }

    func testTheRcSheetReadsTheSameWordsAsTheRcCell() {
        XCTAssertEqual(rcDetail(nil), [])
    }

    func testTheStatusPillHidesOnlyWhenNothingWouldDrawInIt() {
        XCTAssertFalse(statusPillShown(false, false, false))
        XCTAssertTrue(statusPillShown(false, false, true))
        XCTAssertTrue(statusPillShown(true, false, false))
    }
}
