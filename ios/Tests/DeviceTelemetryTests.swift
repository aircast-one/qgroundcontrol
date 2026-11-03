import XCTest
@testable import Aircast

final class DeviceTelemetryTests: XCTestCase {
    func testReadsTheTelemetryViewTheDeviceServes() {
        let view = parseTelemetry(JSON.parse("""
            {"status":{"level":0,"text":"ready"},"fix":{"title":"No flight controller found","steps":["Plug it in.","Power it on."],"waiting":"Looking for it."},
            "notices":[{"level":3,"text":"The MAVLink proxy is down."}],
            "endpoints":[{"proto":"UDP","label":"MAVLink","url":"falcon:14550","down":true}],
            "cloud":{"status":{"level":0,"text":"Carrying"},"text":"Also carrying telemetry."},
            "settings":{"endpoints":"udps:0.0.0.0:14550","baud":"57600"},
            "schemes":[{"value":"udps","label":"udps","host":"0.0.0.0","port":"14550","hostLabel":"Address","placeholder":"","hint":"udps hint"}],
            "gimbal":{"title":"SIYI gimbal camera","status":{"level":0,"text":"connected"},"vendor":"siyi","vendors":[{"value":"siyi","label":"SIYI"}],"models":[]}}
            """))
        XCTAssertEqual(view?.status, DeviceBadge(level: DeviceLevel.Ready, text: "ready"))
        XCTAssertEqual(view?.endpoints.first?.url, "falcon:14550")
        XCTAssertEqual(view?.endpoints.first?.down, true)
        XCTAssertNil(view?.endpointNotice)
        XCTAssertEqual(view?.cloud?.status.text, "Carrying")
        XCTAssertEqual(view?.settings, TelemetrySettings(endpoints: "udps:0.0.0.0:14550", baud: "57600"))
        XCTAssertEqual(view?.schemes[0]["value"].string, "udps")
        XCTAssertEqual(view?.gimbal?.vendors, [DeviceOption(value: "siyi", label: "SIYI")])
        XCTAssertEqual(view?.fix, TelemetryFix(title: "No flight controller found", steps: ["Plug it in.", "Power it on."], waiting: "Looking for it."))
        XCTAssertNil(parseTelemetry(JSON.parse(#"{"status":{"level":0,"text":"ready"}}"#))?.fix)
        XCTAssertNil(parseTelemetry(JSON.parse(#"{"endpoints":[]}"#)))
        let endpoint = DeviceEndpoint(proto: "UDP", label: "MAVLink", url: "falcon:14550", hint: "", note: "", appLink: "", down: false, primary: false)
        XCTAssertEqual(endpointLabel(endpoint), "UDP · MAVLink")
        XCTAssertEqual(
            endpointLabel(DeviceEndpoint(proto: "TCP", label: "TCP · Betaflight", url: "falcon:14550", hint: "", note: "", appLink: "", down: false, primary: false)),
            "TCP · Betaflight"
        )
    }

    func testReadsTheEndpointRowsTheCoreMakesOfTheStoredSpecOrTheTypedRows() {
        let editor = parseEndpointEditor(JSON.parse("""
            {"ok":true,"result":{
            "rows":[{"scheme":"udpc","host":"","port":"14550","hostLabel":"Ground station address","placeholder":"192.168.1.20","note":"Pushes telemetry.",
                     "problem":"Needs an address. It is not saved until it has one.",
                     "options":[{"value":"udps","label":"UDP · listen","row":{"scheme":"udps","host":"0.0.0.0","port":"14550"}}]}],
            "blank":{"scheme":"udps","host":"0.0.0.0","port":"14550","hostLabel":"Listen address","placeholder":"0.0.0.0","note":"Listens.","problem":"","options":[]},
            "endpoints":"udps:0.0.0.0:14550"}}
            """))
        XCTAssertEqual(editor?.rows.count, 1)
        XCTAssertEqual(
            editor?.rows.first,
            EndpointRowView(
                row: EndpointRow(scheme: "udpc", host: "", port: "14550"),
                hostLabel: "Ground station address",
                placeholder: "192.168.1.20",
                note: "Pushes telemetry.",
                problem: "Needs an address. It is not saved until it has one.",
                options: [EndpointOption(value: "udps", label: "UDP · listen", row: EndpointRow(scheme: "udps", host: "0.0.0.0", port: "14550"))]
            )
        )
        XCTAssertEqual(editor?.blank.row, EndpointRow(scheme: "udps", host: "0.0.0.0", port: "14550"))
        XCTAssertEqual(editor?.endpoints, "udps:0.0.0.0:14550")
        XCTAssertNil(parseEndpointEditor(JSON.parse(#"{"ok":false,"reason":"no"}"#)))
        XCTAssertNil(parseEndpointEditor(nil))
    }
}
