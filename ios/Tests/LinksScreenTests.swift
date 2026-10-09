import XCTest
@testable import Aircast

private func written(_ writes: [(String, Any)]) -> [String] { writes.map { "\($0.0)=\(JSON($0.1).text)" } }

final class LinksScreenTests: XCTestCase {
    private func view(_ configured: String...) -> JSON {
        JSON.parse(#"{"configured":[\#(configured.joined(separator: ","))]}"#)
    }

    private let heardTcp = #"""
    {"index":1,"name":"Pi","statusLine":"Connected · TCP 10.0.0.4:5760",
        "connected":true,"heardVehicle":true,"lastError":""}
    """#
    private let waitingUdp = #"""
    {"index":2,"name":"Bench","statusLine":"Waiting for the vehicle · UDP port 14551",
        "connected":true,"heardVehicle":false,"lastError":""}
    """#
    private let idle = #"""
    {"index":3,"name":"Old","statusLine":"Not connected","connected":false,
        "heardVehicle":false,"lastError":"Connection refused"}
    """#

    func testAUdpLinksServerAddressesAndASerialLinksFramingAreReadFromTheRow() {
        let udp = #"{"index":4,"name":"U","statusLine":"","connected":false,"heardVehicle":false,"lastError":"","hostList":["10.0.0.2:14550"]}"#
        let serial = #"{"index":5,"name":"S","statusLine":"","connected":false,"heardVehicle":false,"lastError":"","dataBits":7,"stopBits":2,"parity":3,"flowControl":1}"#
        let rows = linkRows(view(udp, serial))
        XCTAssertEqual(rows[0].servers, ["10.0.0.2:14550"])
        XCTAssertEqual(rows[1].framing, SerialFraming(dataBits: 7, stopBits: 2, parity: 3, flowControl: 1))
        XCTAssertEqual(linkRows(view(heardTcp))[0].framing, SerialFraming())
    }

    func testTheSentenceComesFromTheCoreNotFromTheHead() {
        XCTAssertEqual(
            linkRows(view(heardTcp, waitingUdp, idle)).map(\.statusLine),
            ["Connected · TCP 10.0.0.4:5760", "Waiting for the vehicle · UDP port 14551", "Not connected"]
        )
    }

    func testARowKeepsTheIndexTheCoreGaveItNotItsPositionInTheList() {
        XCTAssertEqual(linkRows(view(heardTcp, waitingUdp, idle)).map(\.index), [1, 2, 3])
    }

    func testHeardIsWhatDecidesEmphasisAndItIsNotTheSameAsConnected() {
        let rows = linkRows(view(heardTcp, waitingUdp))
        XCTAssertEqual(rows.map(\.connected), [true, true])
        XCTAssertEqual(rows.map(\.heard), [true, false])
    }

    func testTheCoreHasAlreadyDroppedTheAutomaticLinks() {
        let withDynamic = JSON.parse(#"{"links":[{"index":0,"name":"UDP Link (AutoConnect)","dynamic":true}],"configured":[\#(heardTcp)]}"#)
        XCTAssertEqual(linkRows(withDynamic).map(\.name), ["Pi"])
    }

    func testALastErrorIsCarriedThrough() {
        XCTAssertEqual(linkRows(view(idle)).map(\.lastError), ["Connection refused"])
    }

    func testAnAbsentViewYieldsNoRows() {
        XCTAssertEqual(linkRows(nil), [])
        XCTAssertEqual(linkRows(JSON.parse("{}")), [])
    }

    func testReadingTheUnfilteredListInsteadOfTheFilteredOneYieldsNothing() {
        XCTAssertTrue(linkRows(JSON.parse(#"{"links":[\#(heardTcp)]}"#)).isEmpty)
    }

    func testAPayloadUsingInventedNamesForTheSentenceLeavesItBlank() {
        let invented = #"{"index":0,"name":"X","status":"Connected","state":"Connected","connected":true}"#
        XCTAssertEqual(linkRows(view(invented)).map(\.statusLine), [""])
    }

    func testALinkNamesItselfFromWhatItPointsAt() {
        XCTAssertEqual(autoLinkName(.Udp, "", "14550"), "UDP 14550")
        XCTAssertEqual(autoLinkName(.Udp, "10.0.0.4", "14550"), "UDP 14550")
        XCTAssertEqual(autoLinkName(.Tcp, "10.0.0.4", "5760"), "TCP 10.0.0.4:5760")
        XCTAssertEqual(autoLinkName(.Tcp, "", "5760"), "TCP")
    }

    func testASuggestedNameAlreadyInUseGetsANumberLikeLinkSettingsUniqueName() {
        XCTAssertEqual(uniqueLinkName("UDP 14550", ["TCP"]), "UDP 14550")
        XCTAssertEqual(uniqueLinkName("UDP 14550", ["UDP 14550"]), "UDP 14550 (2)")
        XCTAssertEqual(uniqueLinkName("UDP 14550", ["UDP 14550", "UDP 14550 (2)"]), "UDP 14550 (3)")
    }

    func testAPortOutsideTheValidRangeIsRefused() {
        XCTAssertEqual(linkFormError(.Udp, "", "0"), "Enter a port between 1 and 65535, or leave it blank for 14550")
        XCTAssertEqual(linkFormError(.Udp, "", "70000"), "Enter a port between 1 and 65535, or leave it blank for 14550")
        XCTAssertEqual(linkFormError(.Udp, "", "abc"), "Enter a port between 1 and 65535, or leave it blank for 14550")
    }

    func testTcpWithoutAnAddressIsRefusedAndUdpWithoutOneIsFine() {
        XCTAssertEqual(linkFormError(.Tcp, "", "5760"), "A TCP link needs the address of the device to call.")
        XCTAssertNil(linkFormError(.Udp, "", "14550"))
    }
}

final class SerialLinkFormTests: XCTestCase {
    private func ports(_ pairs: (String, String)...) -> JSON {
        JSON.parse(#"{"kind":"object","serialPorts":["# + pairs.map { #"{"port":"\#($0.0)","label":"\#($0.1)"}"# }.joined(separator: ",") + "]}")
    }

    func testEachPortKeepsTheLabelTheCorePairedItWith() {
        let choices = serialPortChoices(ports(("/dev/ttyUSB0", "FTDI UART"), ("/dev/ttyACM0", "/dev/ttyACM0")))
        XCTAssertEqual(choices.map(\.label), ["FTDI UART", "/dev/ttyACM0"])
        XCTAssertEqual(choices[0].port, "/dev/ttyUSB0")
    }

    func testABlankLabelOrPortDoesNotBlankTheRow() {
        XCTAssertEqual(serialPortChoices(ports(("/dev/ttyUSB0", ""), ("", "ghost"))), [SerialPortChoice(port: "/dev/ttyUSB0", label: "/dev/ttyUSB0")])
        XCTAssertEqual(serialPortChoices(nil), [])
    }

    func testNoPortPickedIsRefused() {
        XCTAssertEqual(serialFormError("", DEFAULT_BAUD, [], "", true), "Pick the port the radio is plugged into.")
    }

    func testATypedDuplicateNameIsRefusedABlankOneIsNumberedInstead() {
        XCTAssertEqual(serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, ["Serial ttyUSB0"], "Serial ttyUSB0", true), "A link with that name already exists.")
        XCTAssertNil(serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, ["Serial ttyUSB0"], "", true))
    }

    func testAGoodSerialFormHasNothingToSay() {
        XCTAssertNil(serialFormError("/dev/ttyUSB0", DEFAULT_BAUD, ["UDP 14550"], "", true))
    }

    func testTheAutomaticNameIsThePortsLeaf() {
        XCTAssertEqual(autoSerialName("Holybro radio (ttyUSB0)"), "Holybro radio (ttyUSB0)")
        XCTAssertEqual(autoSerialName(""), "Serial", "a port with no display name is suggested as plain Serial, as SerialSettings does")
        XCTAssertEqual(portFor(.Tcp, "14550"), "5760")
        XCTAssertEqual(portFor(.Udp, "14551"), "14551")
        XCTAssertNil(linkFormError(.Udp, "", "", udpDefault: "14551"), "a blank UDP port means the configured listen port")
    }

    func testAnEmptyPortListExplainsItselfRatherThanBlamingTheOperator() {
        XCTAssertEqual(serialFormError("", DEFAULT_BAUD, [], "", false), "Nothing is plugged in. Connect a radio over USB and it will appear here.")
    }
}

final class LinkEditRulesTests: XCTestCase {
    private func row(connected: Bool = false, editing: LinkEditing = .PortOnly) -> LinkRow {
        LinkRow(index: 0, name: "UDP 14550", statusLine: "", connected: connected, heard: false, lastError: "", editing: editing)
    }

    func testAConnectedLinkIsNotEditedUnderneathItself() {
        XCTAssertFalse(linkIsEditable(row(connected: true)))
    }

    func testADisconnectedUdpLinkIsEditable() {
        XCTAssertTrue(linkIsEditable(row()))
    }

    func testAKindTheCoreHasNoFormForIsNotEditable() {
        XCTAssertFalse(linkIsEditable(row(editing: .None)))
    }

    func testEachLinkKindShowsItsOwnIcon() {
        XCTAssertEqual(linkIcon(.Udp), .wifi)
        XCTAssertEqual(linkIcon(.Serial), .usb)
        XCTAssertEqual(linkIcon(.Bluetooth), .bluetooth)
        XCTAssertEqual(linkIcon(.Tcp), .lan)
        XCTAssertEqual(linkIcon(.AircastCloud), .cloud)
        XCTAssertEqual(linkIcon(.LogReplay), .history)
        XCTAssertEqual(linkIcon(.Mock), .science)
        XCTAssertEqual(linkIcon(.Other), .link)
        XCTAssertEqual(linkRows(JSON.parse(#"{"configured":[{"index":0,"type":"tcp"}]}"#)).map(\.type), [.Tcp])
    }

    func testUdpWritesItsOwnPortPropertyAndNotTcps() {
        XCTAssertEqual(written(editWrites(.PortOnly, "n", "", 14551, "", 0, false, false)), [#"name="n""#, "autoConnect=false", "highLatency=false", "localPort=14551"])
    }

    func testTcpWritesHostAndPort() {
        XCTAssertEqual(written(editWrites(.HostAndPort, "n", "1.2.3.4", 5760, "", 0, true, false)), [#"name="n""#, "autoConnect=true", "highLatency=false", #"host="1.2.3.4""#, "port=5760"])
    }

    func testSerialWritesThePortNameBaudAndTheAdvancedFraming() {
        let writes = editWrites(.Serial, "n", "", 0, "/dev/ttyUSB0", 57600, false, true, framing: SerialFraming(dataBits: 7, stopBits: 2, parity: 2, flowControl: 1))
        XCTAssertEqual(written(writes), [
            #"name="n""#, "autoConnect=false", "highLatency=true", #"portName="\/dev\/ttyUSB0""#, "baud=57600",
            "dataBits=7", "stopBits=2", "parity=2", "flowControl=1",
        ])
    }

    func testAnUnknownKindStillRenamesAndWritesNothingElse() {
        XCTAssertEqual(written(editWrites(.None, "n", "h", 1, "p", 2, false, false)), [#"name="n""#, "autoConnect=false", "highLatency=false"])
    }

    func testANewLinksFlagsAreWrittenToItsRow() {
        let writes = linkFlagWrites(3, true, false)
        XCTAssertEqual(writes.map(\.0), ["links.linkConfigurations.3.autoConnect", "links.linkConfigurations.3.highLatency"])
        XCTAssertEqual(writes.map(\.1), [true, false])
    }

    func testARowReadsItsConnectOnStartAndHighLatencyFlags() {
        let rows = linkRows(JSON.parse(#"{"configured":[{"index":0,"name":"n","autoConnect":true,"highLatency":true}]}"#))
        XCTAssertEqual([rows[0].autoConnect, rows[0].highLatency], [true, true])
    }

    func testANewSerialLinkGetsItsFramingWrittenBeforeItConnectsAsSerialSettingsDoesOnAdd() {
        let writes = framingWrites(2, SerialFraming(dataBits: 7, stopBits: 2, parity: 2, flowControl: 1))
        XCTAssertEqual(writes.map(\.0), ["links.linkConfigurations.2.dataBits", "links.linkConfigurations.2.stopBits", "links.linkConfigurations.2.parity", "links.linkConfigurations.2.flowControl"])
        XCTAssertEqual(writes.map(\.1), [7, 2, 2, 1])
    }

    func testASavedBluetoothLinkIsEditedByPickingAnotherDeviceAsBluetoothSettingsDoes() {
        XCTAssertTrue(linkIsEditable(LinkRow(index: 0, name: "HC-05", statusLine: "", connected: false, heard: false, lastError: "", editing: .Device)))
        XCTAssertEqual(
            written(editWrites(.Device, "HC-05", "", 0, "", 0, false, false)),
            [#"name="HC-05""#, "autoConnect=false", "highLatency=false"],
            "the device is set by address through setDeviceByAddress, not written as a field"
        )
    }

    func testASavedLogReplayLinkIsEditedByChoosingAnotherLogAsLogReplaySettingsDoes() {
        XCTAssertTrue(linkIsEditable(LinkRow(index: 0, name: "Replay", statusLine: "", connected: false, heard: false, lastError: "", editing: .LogFile)))
        XCTAssertEqual(
            written(editWrites(.LogFile, "Replay", "", 0, "", 0, false, false, logFile: "/data/r.tlog")),
            [#"name="Replay""#, "autoConnect=false", "highLatency=false", #"filename="\/data\/r.tlog""#]
        )
    }
}

final class UdpServerFormTests: XCTestCase {
    func testABareAddressSendsToTheListeningPortLikeUdpConfigurationAddHost() {
        XCTAssertEqual(udpServer("10.0.0.2", "14550"), "10.0.0.2:14550")
        XCTAssertEqual(udpServer(" 10.0.0.2:14555 ", "14550"), "10.0.0.2:14555")
    }

    func testAnAddressTheConfigurationWouldRefuseIsNotListed() {
        XCTAssertNil(udpServer("a:b:c", "14550"))
        XCTAssertNil(udpServer(":14550", "14550"))
        XCTAssertNil(udpServer("10.0.0.2:70000", "14550"))
    }

    func testAServerAlreadyListedIsNotAddedTwice() {
        let once = withServer([], "10.0.0.2", "14550")
        XCTAssertEqual(withServer(once, "10.0.0.2:14550", "14550"), ["10.0.0.2:14550"])
        XCTAssertEqual(withServer(once, "bad:host:name", "14550"), once)
    }

    func testFormErrorsPointAtTheFieldThatFailed() {
        XCTAssertEqual(linkFormErrorField(.Udp, "", "70000", udpDefault: "14550"), "port")
        XCTAssertEqual(linkFormErrorField(.Tcp, "10.0.0.2", "abc"), "port")
        XCTAssertEqual(linkFormErrorField(.Tcp, "", "5760"), "host")
        XCTAssertNil(linkFormErrorField(.Udp, "", "", udpDefault: "14550"))
        XCTAssertNil(linkFormErrorField(.Tcp, "10.0.0.2", "5760"))
    }

    func testABlankUdpPortFallsBackToTheDefaultPort() {
        XCTAssertNil(linkFormError(.Udp, "", ""))
        XCTAssertEqual(linkFormError(.Udp, "", "0"), "Enter a port between 1 and 65535, or leave it blank for 14550")
    }
}

final class ReplayLogStagingTests: XCTestCase {
    private let files = FileManager.default
    private let link = "Replay test \(UUID().uuidString)"
    private lazy var source = files.temporaryDirectory.appendingPathComponent("replay-source-\(UUID().uuidString)", isDirectory: true)

    override func setUpWithError() throws {
        try files.createDirectory(at: source, withIntermediateDirectories: true)
    }

    override func tearDown() {
        pruneReplayFolder(link, "")
        try? files.removeItem(at: source)
    }

    private func log(_ name: String, _ text: String) throws -> URL {
        let file = source.appendingPathComponent(name)
        try Data(text.utf8).write(to: file)
        return file
    }

    func testStagingCopiesTheLogUnderItsNameIntoAFolderNamedSafelyForTheLink() throws {
        let staged = URL(fileURLWithPath: try XCTUnwrap(stagedReplayLog(link, try log("flight 1.tlog", "first"))))
        XCTAssertEqual(staged.lastPathComponent, "flight 1.tlog")
        XCTAssertEqual(staged.deletingLastPathComponent().lastPathComponent, link.replacingOccurrences(of: " ", with: "_"))
        XCTAssertEqual(try String(contentsOf: staged, encoding: .utf8), "first")
        XCTAssertEqual(try files.contentsOfDirectory(atPath: staged.deletingLastPathComponent().path), ["flight 1.tlog"])
    }

    func testStagingTheSameNameAgainReplacesTheEarlierCopy() throws {
        _ = try XCTUnwrap(stagedReplayLog(link, try log("flight.tlog", "first")))
        try files.removeItem(at: source.appendingPathComponent("flight.tlog"))
        let staged = try XCTUnwrap(stagedReplayLog(link, try log("flight.tlog", "second")))
        XCTAssertEqual(try String(contentsOfFile: staged, encoding: .utf8), "second")
    }

    func testStagingAFileThatIsGoneGivesNothing() {
        XCTAssertNil(stagedReplayLog(link, source.appendingPathComponent("missing.tlog")))
    }

    func testPruningKeepsOnlyTheLogTheLinkStillUses() throws {
        let kept = try XCTUnwrap(stagedReplayLog(link, try log("kept.tlog", "kept")))
        _ = try XCTUnwrap(stagedReplayLog(link, try log("dropped.tlog", "dropped")))
        pruneReplayFolder(link, kept)
        XCTAssertEqual(try files.contentsOfDirectory(atPath: URL(fileURLWithPath: kept).deletingLastPathComponent().path), ["kept.tlog"])
    }
}

