import XCTest
@testable import Aircast

final class DeviceConnectTests: XCTestCase {
    private let served = JSON.parse("""
        {"choices":[{"value":"rtsp","label":"QGroundControl","hides":["WHEP"]},{"value":"whep","label":"Browser or Aircast QGC","hides":["RTSP"]},{"value":"all","label":"Show all","hides":[]}],
        "scope":"Every link reaches this device at falcon.tail.ts.net.",
        "cameras":[{"name":"cam2","status":{"level":2,"text":"unreachable"},"notice":"The link below is still the right one.","fix":{"label":"Fix cam2 →","href":"/streaming?camera=cam2"},
          "targets":[{"proto":"RTSP","label":"RTSP · QGroundControl","url":"rtsp://falcon:8554/cam2","hint":"Paste it.","qr":true,"down":true,"alternatives":[{"label":"Ethernet port (eth0)","url":"rtsp://192.168.1.5:8554/cam2"}]},
                     {"proto":"WHEP","label":"WHEP · Browser","url":"http://falcon:8889/cam2/whep","reach":"http://falcon:8889/cam2/whep","qr":true,"alternatives":[]}]}],
        "telemetry":{"status":{"level":0,"text":"connected"},"targets":[{"proto":"UDP","label":"UDP · MAVLink","url":"falcon:14550","qr":false,"alternatives":[]}]},
        "trouble":{"fallback":[{"proto":"HLS","label":"Web players and Safari","url":"http://falcon:8888/cam2/index.m3u8","tag":"High latency","qr":true,"alternatives":[]}]}}
        """)

    func testReadsTheLinksTheDeviceServes() {
        let view = parseConnect(served)
        let camera = view?.cameras.first
        XCTAssertEqual(camera?.status, DeviceBadge(level: DeviceLevel.Blocked, text: "unreachable"))
        XCTAssertEqual(camera?.fix, PageLink(label: "Fix cam2 →", href: "/streaming?camera=cam2"))
        let rtsp = camera?.targets.first
        XCTAssertEqual(rtsp?.proto, "RTSP")
        XCTAssertEqual(rtsp?.reach, "")
        XCTAssertEqual(rtsp?.tag, "")
        XCTAssertEqual(rtsp?.down, true)
        XCTAssertEqual(rtsp?.qr, true)
        XCTAssertEqual(rtsp?.alternatives, [ConnectAddress(label: "Ethernet port (eth0)", url: "rtsp://192.168.1.5:8554/cam2")])
        XCTAssertEqual(camera?.targets[1].reach, "http://falcon:8889/cam2/whep")
        XCTAssertEqual(view?.telemetry?.status.text, "connected")
        XCTAssertEqual(view?.trouble?.fallback.first?.tag, "High latency")
        XCTAssertNil(view?.trouble?.addresses)
        XCTAssertNil(view?.noCamera)
        XCTAssertNil(parseConnect(JSON.parse(#"{"cameras":[]}"#)))
        XCTAssertEqual(
            parseConnect(JSON.parse(#"{"choices":[],"cameras":[],"noCamera":{"label":"Add a camera","href":"/streaming"}}"#))?.noCamera,
            PageLink(label: "Add a camera", href: "/streaming")
        )
    }

    func testTheGroundStationTheOperatorNamedDecidesWhichLinksAreOnOffer() {
        let view = parseConnect(served)
        let choices = view?.choices ?? []
        let targets = view?.cameras.first?.targets ?? []
        XCTAssertEqual(chosenGcs(choices, nil)?.value, "rtsp")
        XCTAssertEqual(chosenGcs(choices, "nonsense")?.value, "rtsp")
        XCTAssertEqual(shownTargets(targets, chosenGcs(choices, nil)).map(\.proto), ["RTSP"])
        XCTAssertEqual(shownTargets(targets, chosenGcs(choices, "whep")).map(\.proto), ["WHEP"])
        XCTAssertEqual(shownTargets(targets, chosenGcs(choices, "all")).map(\.proto), ["RTSP", "WHEP"])
        XCTAssertEqual(shownTargets(view?.telemetry?.targets ?? [], chosenGcs(choices, "whep")).map(\.proto), ["UDP"])
        XCTAssertNil(chosenGcs([], "rtsp"))
    }

    func testALinkWithNoAddressToProbeGetsNoVerdict() {
        XCTAssertTrue(probeBlocked(""))
        XCTAssertFalse(probeBlocked("http://falcon:8889/cam2/whep"))
        XCTAssertFalse(probeBlocked("https://share.example/w/cam"))
    }
}
