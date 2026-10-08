import XCTest
@testable import Aircast

final class RcControlsTests: XCTestCase {
    func testAnUnsetSettingYieldsNoControls() {
        XCTAssertEqual(parseRcControls("[]"), [])
        XCTAssertEqual(parseRcControls(nil), [])
        XCTAssertEqual(parseRcControls(""), [])
    }

    func testEachSupportedTypeIsRead() {
        let parsed = parseRcControls(#"""
            [{"label":"Gimbal","channel":7,"type":"slider"},
             {"label":"Light","channel":8,"type":"button"},
             {"label":"Mode","channel":9,"type":"switch3"},
             {"label":"Drop","channel":10,"type":"momentary"}]
            """#)
        XCTAssertEqual(parsed, [
            RcControl(label: "Gimbal", channel: 7, type: .Slider),
            RcControl(label: "Light", channel: 8, type: .Button),
            RcControl(label: "Mode", channel: 9, type: .Switch3),
            RcControl(label: "Drop", channel: 10, type: .Momentary),
        ])
    }

    func testAControlWithNoChannelIsDroppedRatherThanSentToChannelZero() {
        XCTAssertEqual(parseRcControls(#"[{"label":"X","type":"slider"}]"#), [])
    }

    func testAnUnknownTypeIsDroppedRatherThanGuessed() {
        XCTAssertEqual(parseRcControls(#"[{"label":"X","channel":5,"type":"dial"}]"#), [])
    }

    func testAControlWithNoLabelFallsBackToItsChannel() {
        XCTAssertEqual(parseRcControls(#"[{"channel":6,"type":"slider"}]"#), [RcControl(label: "CH6", channel: 6, type: .Slider)])
    }

    func testMalformedJsonYieldsNothingRatherThanThrowing() {
        XCTAssertEqual(parseRcControls("not json"), [])
    }
}

final class RcSendRateTests: XCTestCase {
    func testADragDoesNotSendOnEveryPixel() {
        XCTAssertFalse(rcSendDue(1_040, 1_000, false))
        XCTAssertFalse(rcSendDue(1_099, 1_000, false))
    }

    func testADragStillSendsOftenEnoughToFeelLive() {
        XCTAssertTrue(rcSendDue(1_100, 1_000, false))
        XCTAssertTrue(rcSendDue(2_000, 1_000, false))
    }

    func testLettingGoAlwaysSendsSoTheVehicleEndsWhereTheFingerDid() {
        XCTAssertTrue(rcSendDue(1_001, 1_000, true))
        XCTAssertTrue(rcSendDue(1_000, 1_000, true))
    }

    func testTheFirstSendOfADragIsNeverThrottled() {
        XCTAssertTrue(rcSendDue(5_000, 0, false))
    }
}
