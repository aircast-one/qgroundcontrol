import XCTest
@testable import Aircast

private func view(_ live: Int...) -> JSON {
    let slots = ["Acro", "AltHold", "Auto", "Guided", "Loiter", "RTL"].enumerated().map { index, mode in
        #"{"slot": \#(index + 1), "mode": "\#(mode)", "live": \#(live.contains(index + 1))}"#
    }.joined(separator: ",")
    return JSON.parse(#"{"available": true, "channel": 5, "liveSlot": \#(live.first ?? 0), "reason": "", "slots": [\#(slots)]}"#)
}

final class ModeSlotsTests: XCTestCase {
    func testTheLiveSlotIsNamedByItsNumberAndItsModeWhichIsWhatThePageExistsToAnswer() {
        XCTAssertEqual(liveSlotText(modeSlotsView(view(4))), "The switch on channel 5 is on slot 4, Guided.")
    }

    func testASwitchSittingBetweenTheBandsSaysSoRatherThanNamingASlot() {
        XCTAssertEqual(liveSlotText(modeSlotsView(view())), "The switch on channel 5 is not on any mode slot.")
    }

    func testAnUnassignedModeChannelSaysSoInsteadOfNamingChannel0() {
        let unassigned = JSON.parse(view().text.replacingOccurrences(of: #""reason":"""#, with: #""reason":"No mode channel is assigned.""#))
        XCTAssertEqual(liveSlotText(modeSlotsView(unassigned)), "No mode channel is assigned.")
    }

    func testAVehicleThatDoesNotPickModesFromAChannelHasNothingToShow() {
        XCTAssertNil(modeSlotsView(JSON.parse(#"{"available": false, "slots": [], "liveSlot": 0}"#)))
        XCTAssertNil(liveSlotText(nil))
    }

    func testLiveSwitchesAndChannelOptionsAreNamedLikeTheHighlightedRowsInQgcsFlightModePages() {
        let px4 = modeSlotsView(JSON.parse(#"{"available":true,"channel":5,"slots":[],"activeSwitches":["Kill switch channel"],"channelOptions":[]}"#))
        XCTAssertEqual(liveSwitchesText(px4), "Switches on: Kill switch channel")
        let apm = modeSlotsView(JSON.parse(#"{"available":true,"channel":5,"slots":[],"channelOptions":[{"channel":7,"enabled":true},{"channel":8,"enabled":false}]}"#))
        XCTAssertEqual(liveSwitchesText(apm), "Channel options on: channel 7")
        XCTAssertNil(liveSwitchesText(modeSlotsView(JSON.parse(#"{"available":true,"channel":5,"slots":[]}"#))))
    }

    func testTheChannelMonitorIsShownWhereTheCoreSaysSoPx4sFlightModesPage() {
        XCTAssertEqual(modeSlotsView(JSON.parse(#"{"available":true,"channel":5,"slots":[],"channelMonitor":true}"#))?.channelMonitor, true)
        XCTAssertEqual(modeSlotsView(JSON.parse(#"{"available":true,"channel":5,"slots":[]}"#))?.channelMonitor, false)
    }

    func testTheRowsToPaintAreTheOnesTheCoreNames() {
        let view = modeSlotsView(JSON.parse(#"{"available":true,"activeParams":["FLTMODE4","RC7_OPTION"]}"#))!
        XCTAssertEqual(view.activeParams, ["FLTMODE4", "RC7_OPTION"])
        XCTAssertEqual(modeSlotsView(JSON.parse(#"{"available":true}"#))!.activeParams, [])
    }
}
