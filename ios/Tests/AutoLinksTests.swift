import XCTest
@testable import Aircast

final class AutoLinksTests: XCTestCase {
    func testConnectedAutomaticLinksAreListedSavedAndIdleOnesAreNot() {
        let view = JSON.parse(#"""
        {"links":[
            {"name":"UDP Link (AutoConnect)","displaySummary":"UDP port 14550","dynamic":true,"connected":true,"heardVehicle":false},
            {"name":"TCP 127.0.0.1:5771","displaySummary":"127.0.0.1:5771","dynamic":true,"connected":true,"heardVehicle":true},
            {"name":"SiK","displaySummary":"ttyUSB0","dynamic":false,"connected":true,"heardVehicle":true},
            {"name":"Old","displaySummary":"x","dynamic":true,"connected":false,"heardVehicle":false}]}
        """#)
        let links = autoLinks(view)
        XCTAssertEqual(links.map(\.name), ["UDP Link (AutoConnect)", "TCP 127.0.0.1:5771"])
        XCTAssertEqual(links.map(autoLinkStatus), ["Listening", "Vehicle"])
    }

    func testALinkWithNoAddressReadsWithoutAStraySeparatorAndAMockLinkReadsAsSimulated() {
        XCTAssertEqual(autoLinkSubtitle(AutoLink(name: "Serial", summary: "", heard: false)), "automatic")
        XCTAssertEqual(autoLinkSubtitle(AutoLink(name: "UDP", summary: "UDP port 14550", heard: false)), "UDP port 14550 \u{00b7} automatic")
        XCTAssertEqual(autoLinkSubtitle(AutoLink(name: "PX4 MultiRotor MockLink", summary: "", heard: true, type: .Mock)), "Simulated")
    }
}
