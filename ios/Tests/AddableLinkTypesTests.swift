import XCTest
@testable import Aircast

final class AddableLinkTypesTests: XCTestCase {
    private func links(_ ids: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"Links","links":[],"linkTypeIds":\#(ids)}"#)
    }

    func testABuildWithoutSerialSupportStopsOfferingSerial() {
        XCTAssertEqual(
            addableLinkTypes(links(#"["udp","tcp"]"#)),
            [.Udp, .Tcp],
            "linkTypeTable() wraps serial in QGC_NO_SERIAL_LINK, and createSerialConfiguration returns false on such a build"
        )
    }

    func testATypeThisHeadCannotCreateIsNotOfferedJustBecauseTheCoreListsIt() {
        XCTAssertEqual(
            addableLinkTypes(links(#"["serial","udp","tcp","bluetooth"]"#)),
            [.Udp, .Tcp, .Serial],
            "createAndConnectLink handles udp and tcp and returns false for anything else, and Bluetooth is only offered where the core has a Bluetooth host"
        )
    }

    func testACoreTooOldToServeTheListKeepsAllThreeRatherThanOfferingNone() {
        XCTAssertEqual(addableLinkTypes(JSON.parse(#"{"kind":"object"}"#)), CREATABLE_LINK_TYPES)
        XCTAssertEqual(addableLinkTypes(nil), CREATABLE_LINK_TYPES)
        XCTAssertEqual(addableLinkTypes(links("[]")), CREATABLE_LINK_TYPES)
    }

    func testBluetoothIsOfferedWhenTheCoreHasABluetoothHost() {
        let view = JSON.parse(#"{"kind":"object","linkTypeIds":["serial","udp","tcp","bluetooth"],"bluetooth":{"available":true,"scanning":false,"devices":[{"name":"HC-05","address":"98:D3:31:F6:12:34"}]}}"#)
        XCTAssertEqual(addableLinkTypes(view), [.Udp, .Tcp, .Serial, .Bluetooth])
        XCTAssertEqual(bluetoothState(view).devices, [BluetoothDeviceChoice(name: "HC-05", address: "98:D3:31:F6:12:34")])
    }

    func testLogReplayIsOfferedWhenTheCoreListsItAfterTheLiveLinks() {
        XCTAssertEqual(addableLinkTypes(links(#"["serial","udp","tcp","logReplay"]"#)), [.Udp, .Tcp, .Serial, .LogReplay])
    }

    func testEachTypeIsNamedInFullAsLinkManagersTypeStringsAre() {
        XCTAssertEqual(
            [LinkType.Udp, .Tcp, .Serial, .Bluetooth, .LogReplay, .AircastCloud].map(linkTypeLabel),
            ["UDP", "TCP", "Serial", "Bluetooth", "Log replay", "Aircast Cloud"]
        )
    }

    func testAddLinkListsPilotLinksFirstPutsAPluggedInUsbRadioOnTopAndKeepsReplayAndSimulationApart() {
        let offered: [LinkType] = [.Udp, .Tcp, .Serial, .Bluetooth, .LogReplay, .Mock, .AircastCloud]
        XCTAssertEqual(pilotLinkKinds(offered, false), [.Udp, .Serial, .Bluetooth, .AircastCloud, .Tcp])
        XCTAssertEqual(pilotLinkKinds(offered, true), [.Serial, .Udp, .Bluetooth, .AircastCloud, .Tcp])
        XCTAssertEqual(toolLinkKinds(offered), [.LogReplay, .Mock])
        XCTAssertEqual(pilotLinkKinds([.Udp, .LogReplay], true), [.Udp])
    }

    func testEveryLinkKindHasAPilotTitleAnExplanationAndASubtitleWithinThe32CharacterRowBudget() {
        (PILOT_LINK_ORDER + TOOL_LINK_ORDER).map(linkKind).forEach { kind in
            XCTAssertTrue(!kind.title.isBlank && !kind.about.isBlank && kind.detail.count <= 32, kind.type.id)
        }
    }
}
