import XCTest
@testable import Aircast

final class DeviceNetworkTests: XCTestCase {
    private let served = JSON.parse("""
        {
          "uplink": {
            "icon": "wifi",
            "title": "Wi-Fi (wlan0)",
            "status": { "level": 0, "text": "carrying traffic" },
            "verdict": { "level": 3, "text": "No video going to the cloud" },
            "addr": "192.168.1.20/24",
            "failover": [{ "label": "LTE (wwan0)", "status": { "level": 3, "text": "standing by · takes over in ~15 s" } }],
            "switch": "Moved just now: from Wi-Fi (wlan0) to LTE (wwan0)"
          },
          "modem": {
            "status": { "level": 0, "text": "connected" },
            "summary": "HARNESS",
            "notices": [],
            "operator": "HARNESS",
            "detail": "lte · home · signal 72%",
            "signal": { "label": "Signal", "values": [80, -1, 72], "max": 100, "value": "72%", "level": 3 },
            "facts": [{ "label": "APN (in use)", "value": "internet" }],
            "messages": { "summary": "1 on the SIM · latest from MAGTICOM", "items": [{ "from": "MAGTICOM", "text": "code 2157", "when": "Sep 25 17:41" }] },
            "sim": { "summary": "APN internet", "apn": "internet", "placeholder": "auto", "metered": "auto", "options": [{ "value": "auto", "label": "Automatic" }] }
          },
          "starlink": { "status": { "level": 3, "text": "no dish" }, "summary": "", "notices": [], "facts": [], "address": { "value": "10.0.0.9:9200", "default": "192.168.100.1:9200", "defaultFirst": true, "open": true } },
          "wifi": { "status": { "level": 3, "text": "not joined" }, "summary": "join a network", "reopen": { "host": "harness.local", "hostname": "harness" } },
          "interfaces": {
            "status": { "level": 3, "text": "" },
            "summary": "1 in use · 1 known",
            "links": [
              {
                "name": "camera (eth0)",
                "kind": "ethernet",
                "where": "eth0 · 192.168.8.2/24",
                "local": true,
                "editor": { "interface": "eth0", "ipv4": { "method": "manual", "metric": "", "metricPlaceholder": "automatic", "address": "192.168.8.2/24", "gateway": "", "dns": "" }, "uuid": "uuid-eth0", "deleteConfirm": "Delete?" }
              }
            ],
            "saved": [{ "uuid": "uuid-spare", "name": "spare-net", "kind": "ethernet", "ipv4": { "method": "auto" }, "activateConfirm": "Activate spare-net?", "deleteConfirm": "Delete spare-net?" }],
            "other": [],
            "methods": [{ "value": "auto", "label": "DHCP (auto)" }],
            "add": { "types": [{ "value": "ethernet", "label": "Ethernet" }], "ports": [], "autoconnect": [] }
          },
          "data": { "status": { "level": 3, "text": "no caps" }, "summary": "0 B this month", "budget": { "summary": "no caps set", "monthlyCapGB": "", "flightCapMB": "", "warnPercent": "0" } },
          "tailscale": {
            "status": { "level": 1, "text": "signing in" },
            "summary": "finish the sign-in",
            "banner": { "title": "Waiting for sign-in", "subtitle": "Step 1 of 2", "status": { "level": 1, "text": "pending" } },
            "facts": [],
            "addresses": [],
            "signIn": { "label": "Open Tailscale login", "href": "https://login.tailscale.com/a/x" },
            "cancel": true
          },
          "relay": { "status": { "level": 3, "text": "off" }, "summary": "", "on": false, "configured": false, "reason": "Add a relay server first", "notice": { "level": 3, "text": "Video keeps working." }, "servers": [] }
        }
        """)

    private func servedWith(_ key: String, _ value: JSON?) -> JSON {
        var map = served.object ?? [:]
        map[key] = value
        return .object(map)
    }

    func testReadsTheManualWiFiControlsTheDeviceOffers() throws {
        let wifi = JSON.parse("""
            {
              "status": { "level": 1, "text": "join failed" }, "summary": "workshop",
              "held": { "level": 2, "text": "Hotspot kept on by hand" },
              "saved": { "label": "Join workshop", "confirm": "Join workshop now?" },
              "hotspot": { "label": "Use hotspot", "confirm": "Keep the hotspot on?" },
              "reopen": { "host": "harness.local", "hostname": "harness" }
            }
            """)
        let view = try XCTUnwrap(parseNetwork(servedWith("wifi", wifi)))
        XCTAssertEqual(view.wifi.held?.text, "Hotspot kept on by hand")
        XCTAssertEqual(view.wifi.saved, ConfirmedAction(label: "Join workshop", confirm: "Join workshop now?"))
        XCTAssertEqual(view.wifi.hotspot?.label, "Use hotspot")
    }

    func testReadsTheNetworkViewTheDeviceServes() throws {
        let view = try XCTUnwrap(parseNetwork(served))
        XCTAssertEqual(view.uplink.failover.first, UplinkBackup(label: "LTE (wwan0)", status: DeviceBadge(level: DeviceLevel.Neutral, text: "standing by · takes over in ~15 s")))
        XCTAssertTrue(view.uplink.switchText.contains("from Wi-Fi (wlan0) to LTE (wwan0)"))
        XCTAssertEqual(view.modem?.signal?.values, [80.0, -1.0, 72.0])
        XCTAssertEqual(view.modem?.messages.items.first?.from, "MAGTICOM")
        XCTAssertEqual(view.modem?.sim?.apn, "internet")
        XCTAssertEqual(view.starlink.address, StarlinkAddress(value: "10.0.0.9:9200", fallback: "192.168.100.1:9200", defaultFirst: true, open: true))
        XCTAssertNil(view.starlink.demo)
        XCTAssertNil(view.wifi.joined)
        XCTAssertNil(view.wifi.held)
        XCTAssertNil(view.wifi.hotspot)
        XCTAssertNil(view.wifi.saved)
        XCTAssertEqual(view.wifi.reopen, WifiReopen(host: "harness.local", hostname: "harness"))
        XCTAssertEqual(view.interfaces.links.first?.editor?.iface, "eth0")
        XCTAssertNil(view.interfaces.links.first?.role)
        XCTAssertEqual(view.interfaces.saved.first?.ipv4.method, "auto")
        XCTAssertNil(view.data.gauge)
        XCTAssertEqual(view.tailscale.signIn?.href, "https://login.tailscale.com/a/x")
        XCTAssertNil(view.tailscale.control)
        XCTAssertEqual(view.relay.reason, "Add a relay server first")
        XCTAssertNotNil(parseNetwork(servedWith("modem", nil)))
        XCTAssertNil(parseNetwork(servedWith("modem", nil))?.modem)
        XCTAssertNil(parseNetwork(.object(["modem": served["modem"]])))
    }

    func testEachKindOfUplinkGetsAnIconOfItsOwnAnythingElseTheNetworkOne() {
        XCTAssertEqual(uplinkIcon("satellite-dish"), .satelliteAlt)
        XCTAssertEqual(uplinkIcon("signal"), .signalCellularAlt)
        XCTAssertEqual(uplinkIcon("wifi"), .wifi)
        XCTAssertEqual(uplinkIcon("ethernet-port"), .usb)
        XCTAssertEqual(uplinkIcon("network"), .lan)
        XCTAssertEqual(uplinkIcon(""), .lan)
    }

    func testRelayServersAreSentAsTheLinesTheCoreMakesOfThemAndNotAtAllWhenItRefuses() {
        XCTAssertEqual(parseIceServerLines(.object(["ok": .bool(true), "result": .string(GOOGLE_STUN)])), GOOGLE_STUN)
        XCTAssertEqual(parseIceServerLines(.object(["ok": .bool(true), "result": .string("")])), "")
        XCTAssertNil(parseIceServerLines(.object(["ok": .bool(false), "reason": .string("no")])))
        XCTAssertNil(parseIceServerLines(nil))
    }

    func testAnEmptyServerListStillOffersOneRowToFillIn() {
        XCTAssertEqual(iceRows([]), [IceServer(url: "", username: "", password: "")])
        let saved = [IceServer(url: GOOGLE_STUN, username: "", password: "")]
        XCTAssertEqual(iceRows(saved), saved)
    }

    func testANewConnectionShowsOnlyTheFieldsItsTypeNeedsAndManualAddressingOnlyWhenChosen() {
        XCTAssertTrue(connectionFieldShown("wifi", "ssid"))
        XCTAssertFalse(connectionFieldShown("wifi", "apn"))
        XCTAssertTrue(connectionFieldShown("gsm", "pin"))
        XCTAssertFalse(connectionFieldShown("ethernet", "psk"))
        XCTAssertTrue(connectionFieldShown("ethernet", "name"))
        XCTAssertTrue(manualAddressing("manual"))
        XCTAssertFalse(manualAddressing("auto"))
        let fields = [
            DeviceField(name: "method", label: "IPv4 method", value: "auto"),
            DeviceField(name: "address", label: "Address", value: "", shownWhen: { manualAddressing($0["method"] ?? "") }),
        ]
        XCTAssertEqual(shownFields(fields, ["method": "auto"]).map(\.name), ["method"])
        XCTAssertEqual(shownFields(fields, ["method": "manual"]).map(\.name), ["method", "address"])
    }
}
