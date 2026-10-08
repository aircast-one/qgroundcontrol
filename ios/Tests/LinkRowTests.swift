import XCTest
@testable import Aircast

private let TWO_LINKS = #"""
{"available": true, "class": "Links", "configured": [
  {"index": 0, "name": "Packet radio", "type": "udp", "typeLabel": "UDP",
   "statusLine": "Connected · UDP port 14550", "connected": true, "heardVehicle": true,
   "goneQuiet": false, "dynamic": false, "lastError": "", "editing": "portOnly",
   "summary": "UDP port 14550", "displaySummary": "UDP port 14550", "path": "links.linkConfigurations.0"},
  {"index": 1, "name": "UDP 14551", "type": "udp", "typeLabel": "UDP",
   "statusLine": "Not hearing the vehicle · UDP port 14551", "connected": true, "heardVehicle": true,
   "goneQuiet": true, "dynamic": false, "lastError": "", "editing": "portOnly",
   "summary": "UDP port 14551", "displaySummary": "UDP port 14551", "path": "links.linkConfigurations.1"}
]}
"""#

final class LinkRowTests: XCTestCase {
    func testALinkThatHasGoneQuietIsCarriedThroughNotCollapsedIntoHeard() throws {
        let quiet = try XCTUnwrap(linkRows(JSON.parse(TWO_LINKS)).first { $0.name == "UDP 14551" })
        XCTAssertTrue(quiet.heard, "heardVehicle stays true once a packet has ever arrived")
        XCTAssertTrue(quiet.goneQuiet, "so the row needs the core's separate answer to tell the operator")
        XCTAssertEqual(quiet.statusLine, "Not hearing the vehicle · UDP port 14551")
    }

    func testTheLinkStillCarryingTheVehicleIsNotMarkedQuiet() throws {
        let live = try XCTUnwrap(linkRows(JSON.parse(TWO_LINKS)).first { $0.name == "Packet radio" })
        XCTAssertTrue(live.heard)
        XCTAssertFalse(live.goneQuiet)
    }
}

final class RemedyTests: XCTestCase {
    func testAnAddressThatNothingAnswersSaysRetryingWillNotHelp() {
        XCTAssertEqual(remedyText(REMEDY_EDIT_ADDRESS), "Nothing is listening at that address. Retrying will not help until it is changed.")
    }

    func testARetryableErrorAddsNoAdviceBecauseConnectAlreadySaysIt() {
        XCTAssertNil(remedyText("retry"))
        XCTAssertNil(remedyText(""))
        XCTAssertNil(remedyText("somethingTheCoreAddedLater"))
    }
}
