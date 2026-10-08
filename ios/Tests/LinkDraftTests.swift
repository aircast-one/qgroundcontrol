import XCTest
@testable import Aircast

final class LinkDraftTests: XCTestCase {
    private let plugged = [SerialPortChoice(port: "/dev/ttyUSB0", label: "Holybro radio (ttyUSB0)")]

    func testALinkThatSavedButCouldNotConnectCountsAsAddedNotAsAFailure() {
        XCTAssertEqual(addOutcome(true, true), .Connected)
        XCTAssertEqual(addOutcome(false, true), .SavedNotConnected)
        XCTAssertEqual(addOutcome(false, false), .Failed)
    }

    func testOnlyALinkThatWasNeverSavedBlamesTheName() {
        XCTAssertEqual(addFailure(.Tcp), "Could not add that link. The name may already be in use.")
        XCTAssertEqual(addFailure(.Mock), "Could not start the simulated vehicle.")
    }

    func testEachKindOfNewLinkAsksForWhatItCannotWorkWithout() {
        XCTAssertEqual(draftError(LinkDraft(type: .Tcp, port: "5760"), "14550", [], false), "A TCP link needs the address of the device to call.")
        XCTAssertNil(draftError(LinkDraft(type: .Tcp, host: "10.0.0.2", port: "5760"), "14550", [], false))
        XCTAssertNil(draftError(LinkDraft(type: .Udp), "14550", [], false))
        XCTAssertEqual(draftError(LinkDraft(type: .Bluetooth), "14550", [], false), "Pick a Bluetooth device.")
        XCTAssertEqual(draftError(LinkDraft(type: .LogReplay), "14550", [], false), "Choose a log file to replay.")
        XCTAssertEqual(draftError(LinkDraft(type: .Serial), "14550", [], true), "Pick the port the radio is plugged into.")
        XCTAssertEqual(draftError(LinkDraft(type: .AircastCloud), "14550", [], false), "")
        XCTAssertNil(draftError(LinkDraft(type: .Mock), "14550", [], false))
    }

    func testASinglePluggedInRadioIsPickedForYouAndAChoiceSurvivesAnotherRadioAppearing() {
        XCTAssertEqual(LinkDraft(type: .Serial).withPluggedPort(plugged).portName, "/dev/ttyUSB0")
        XCTAssertEqual(LinkDraft(type: .Serial).withPluggedPort(plugged + [SerialPortChoice(port: "/dev/ttyACM0", label: "Pixhawk")]).portName, "")
        XCTAssertEqual(LinkDraft(type: .Serial, portName: "/dev/ttyACM0").withPluggedPort(plugged).portName, "/dev/ttyACM0")
    }

    func testANewLinkIsNamedAfterWhatItPointsAt() {
        XCTAssertEqual(suggestedLinkName(LinkDraft(type: .Udp), [], "14550"), "UDP 14550")
        XCTAssertEqual(suggestedLinkName(LinkDraft(type: .Tcp, host: "10.0.0.2"), [], "14550"), "TCP 10.0.0.2:5760")
        XCTAssertEqual(suggestedLinkName(LinkDraft(type: .Serial, portName: "/dev/ttyUSB0"), plugged, "14550"), "Holybro radio (ttyUSB0)")
        XCTAssertEqual(suggestedLinkName(LinkDraft(type: .LogReplay), [], "14550"), REPLAY_LINK_NAME)
    }

    func testLinkKindsAndEditFormsRoundTripTheCoresStringsAndAnythingNewReadsAsOtherOrNone() {
        let known = LinkType.allCases.filter { $0 != .Other }
        XCTAssertEqual(known.map { LinkType.from($0.id) }, known)
        XCTAssertEqual(LinkType.from("satellite"), .Other)
        XCTAssertEqual(LinkType.from(""), .Other)
        XCTAssertEqual(LinkEditing.from("portOnly"), .PortOnly)
        XCTAssertEqual(LinkEditing.from(""), LinkEditing.None)
    }
}

final class LinkEditTests: XCTestCase {
    private func row(editing: LinkEditing = .PortOnly, autoConnect: Bool = false, servers: [String] = [], framing: SerialFraming = SerialFraming()) -> LinkRow {
        LinkRow(index: 0, name: "UDP 14550", statusLine: "", connected: false, heard: false, lastError: "", editing: editing, port: 14550, framing: framing, servers: servers, autoConnect: autoConnect)
    }

    private func edited(_ change: (inout LinkEdit) -> Void) -> LinkEdit {
        var edit = linkEdit(row())
        change(&edit)
        return edit
    }

    func testEditOpensAdvancedWhenTheLinkAlreadyUsesSomethingInIt() {
        XCTAssertFalse(editShowsAdvanced(row()))
        XCTAssertTrue(editShowsAdvanced(row(autoConnect: true)))
        XCTAssertTrue(editShowsAdvanced(row(servers: ["10.0.0.9:14550"])))
        XCTAssertTrue(editShowsAdvanced(row(editing: .Serial, framing: SerialFraming(dataBits: 7))))
    }

    func testEditChecksOnlyTheFieldThatLinkKindHas() {
        XCTAssertNil(editError(.PortOnly, linkEdit(row()), "14550"))
        XCTAssertEqual(editError(.PortOnly, edited { $0.port = "0" }, "14550"), "Enter a port between 1 and 65535, or leave it blank for 14550")
        XCTAssertEqual(editError(.HostAndPort, edited { $0.port = "abc" }, "14550"), "Port must be a number between 1 and 65535.")
        XCTAssertEqual(editError(.LogFile, edited { $0.logFile = "" }, "14550"), "Choose a log file.")
        XCTAssertNil(editError(.Serial, linkEdit(row()), "14550"))
    }

    func testABlankUdpPortSavesAsTheConfiguredListenPort() {
        XCTAssertEqual(editedPort(.PortOnly, "", "14551"), 14551)
        XCTAssertEqual(editedPort(.HostAndPort, "", "14551"), 0)
    }

    func testALinkWhoseAddressIsWrongOffersEditingFirstAndARetrySecond() {
        XCTAssertEqual(primaryLinkAction(true, false), "Disconnect")
        XCTAssertEqual(primaryLinkAction(false, true), "Try again")
        XCTAssertEqual(primaryLinkAction(false, false), "Connect")
    }

    func testTheSettingsSheetHidesItsTabsInsideAPageOrAircraftSetup() {
        XCTAssertFalse(sheetDrilled(nil, false))
        XCTAssertTrue(sheetDrilled("Connections", false))
        XCTAssertTrue(sheetDrilled(nil, true))
    }
}
