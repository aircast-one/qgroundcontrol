import XCTest
@testable import Aircast

final class CommandRefusalTests: XCTestCase {
    func testAReplyTheCoreAlreadyWordedAsABannerIsNotRepeatedInTheMessageBanner() {
        let reply = VehicleMessage(index: 0, time: "12:00:00", severity: "Warning", level: .Warning, text: "No such mode 26", toldElsewhere: true)
        let fence = VehicleMessage(index: 1, time: "12:00:00", severity: "Warning", level: .Warning, text: "Fence breached")
        XCTAssertEqual(bannerMessages([reply, fence]), [fence])
        let read = vehicleMessages(JSON.parse(#"{"order":"oldestFirst","items":[{"index":0,"text":"Arm: Mode not armable","level":"error","toldElsewhere":true},{"index":1,"text":"Crash: Disarming","level":"error"}]}"#))
        XCTAssertEqual(bannerMessages(read).map(\.text), ["Crash: Disarming"], "the core's flag travels into the head")
    }

    func testSentIsABriefAcknowledgementGoneBeforeARefusalCouldSitUnderIt() {
        XCTAssertEqual(SENT_SHOWN_MS, 3_000)
    }
}
