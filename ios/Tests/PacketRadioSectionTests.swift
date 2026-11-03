#if os(macOS)
import XCTest
@testable import Aircast

final class PacketRadioSectionTests: XCTestCase {
    func testLinkQualityReadsLikePacketRadioSettingsQml() throws {
        let status = try XCTUnwrap(packetRadioStatus(JSON.parse(#"""
            {"class":"PacketRadio","statusText":"Receiving on ALFA","linkActive":true,"haveSignal":true,
            "antennaRssiRaw":[60,70],"antennaSnr":[20,25],"linkScore":1800,"packetLoss":2,"videoPackets":1234,"adapters":["ALFA [1]"]}
            """#)))
        XCTAssertEqual(status.signal, "60  |  70")
        XCTAssertEqual(status.noise, "20  |  25 dB")
        XCTAssertEqual(status.linkScore, "1800")
        XCTAssertEqual(status.packetLoss, "2")
        XCTAssertEqual(status.videoPackets, "1234")
        XCTAssertEqual(status.adapters, ["ALFA [1]"])
        let quiet = try XCTUnwrap(packetRadioStatus(JSON.parse(#"{"class":"PacketRadio","statusText":"Listening","linkActive":true,"haveSignal":false}"#)))
        XCTAssertEqual(quiet.signal, "waiting")
        XCTAssertEqual(quiet.linkScore, "waiting")
        XCTAssertEqual(quiet.packetLoss, "")
        XCTAssertNil(packetRadioStatus(JSON.parse(#"{"kind":"null"}"#)))
        XCTAssertNil(packetRadioStatus(nil))
    }

    func testABlankStatusReadsAsUnavailable() throws {
        let status = try XCTUnwrap(packetRadioStatus(JSON.parse(#"{"class":"PacketRadio","statusText":""}"#)))
        XCTAssertEqual(status.statusText, "Unavailable")
        XCTAssertFalse(status.linkActive)
    }

    func testTheAdapterMenuOffersAutomaticFirstAndAutomaticWritesBlank() throws {
        let status = try XCTUnwrap(packetRadioStatus(JSON.parse(#"{"class":"PacketRadio","adapters":["ALFA [1]","RTL [2]"]}"#)))
        XCTAssertEqual(packetRadioAdapterChoices(status), ["Automatic", "ALFA [1]", "RTL [2]"])
        XCTAssertEqual(packetRadioAdapterChoices(nil), ["Automatic"])
        XCTAssertEqual(packetRadioAdapterWrite(0, "Automatic"), "")
        XCTAssertEqual(packetRadioAdapterWrite(2, "RTL [2]"), "RTL [2]")
    }
}
#endif
