import XCTest
@testable import Aircast

final class DeviceCameraWizardTests: XCTestCase {
    func testReadsTheWizardTheDeviceServes() throws {
        let view = parseCameraWizard(
            JSON.parse("""
                {"name":"cam2","title":"cam2 is not answering","new":false,"source":"rtsp://192.168.8.10:554/stream","rtsp":"","rtspHint":"rtsp://192.168.8.10:554/stream","interface":"eth0",
                 "checks":["The camera has power — its light is on"],
                 "interfaces":[{"value":"eth0","label":"Ethernet port (eth0) — 192.168.8.2/24","address":"192.168.8.2/24"}],
                 "piCamera":{"note":"A Raspberry Pi camera is connected to this device.","action":"Use it"},
                 "others":[{"name":"cam","source":"rtsp://x"}]}
                """)
        )
        XCTAssertEqual(view?.iface, "eth0")
        XCTAssertEqual(view?.title, "cam2 is not answering")
        XCTAssertEqual(view?.checks, ["The camera has power — its light is on"])
        XCTAssertEqual(parseCameraWizard(JSON.parse(#"{"name":"cam"}"#))?.checks, [])
        let dark = try XCTUnwrap(parseCameraWizard(JSON.parse(#"{"name":"cam","new":false,"lookingFor":"The camera refuses the connection."}"#)))
        XCTAssertTrue(repairing(dark))
        XCTAssertFalse(repairing(try XCTUnwrap(parseCameraWizard(JSON.parse(#"{"name":"cam","new":true,"lookingFor":"x"}"#)))))
        XCTAssertFalse(nextOffered(dark, "", picked: false))
        XCTAssertTrue(nextOffered(dark, "", picked: true))
        XCTAssertTrue(nextOffered(dark, "rtsp://x", picked: false))
        XCTAssertTrue(nextOffered(try XCTUnwrap(parseCameraWizard(JSON.parse(#"{"name":"cam","new":true}"#))), "", picked: false))
        XCTAssertEqual(view?.interfaces.first?.address, "192.168.8.2/24")
        XCTAssertEqual(view?.piCamera?.action, "Use it")
        XCTAssertEqual(view?.json["others"][0]["name"].string, "cam")
        XCTAssertNil(parseCameraWizard(nil))
    }

    func testReadsWhatTheCoreMadeOfTheScanThePortsAndTheRunningSetup() throws {
        let wizard = parseDeviceWizard(
            JSON.parse("""
                {"session":3,"scanning":false,"scans":2,"again":7,"note":"","checklist":"No camera answered — check it, then scan again",
                 "adopt":{"host":"192.168.8.10","url":"rtsp://192.168.8.10:554/stream1","needsAuth":false,"pathFound":true,"used":"","note":"ready to use","action":"Use"},
                 "cams":[],"camHeading":"0 cameras","camsHint":"","details":"Details for a technician","todoHeading":"Try this",
                 "todo":[{"iface":"eth0","label":"Ethernet port (eth0)","detail":"no address on this port","open":true,"suggest":"192.168.144.2/24","hint":"Prefilled.","go":"Set up eth0 and find the camera",
                   "options":[{"cidr":"192.168.144.2/24","text":"192.168.144.2/24 · found 192.168.144.25"}],"busy":true,"status":"Setting up eth0…"}],
                 "rest":[{"iface":"wlan0","label":"Wi-Fi (wlan0)","detail":"this is the device's own uplink"}],"restTitle":"Wi-Fi (wlan0) — nothing answered",
                 "templates":[{"id":"siyi","label":"SIYI camera","sub":"ZT30","action":"Set up"}],"templateBusy":false,"templateNote":"",
                 "template":{"n":1,"refusal":"","camera":null,"name":"siyi"},
                 "apply":{"n":0,"state":"failed","text":"That name is taken","field":"name","url":"","camera":""},
                 "notice":{"n":1,"text":"Gimbal control was not switched to this camera — no answer"}}
                """)
        )
        XCTAssertEqual(wizard.session, 3)
        XCTAssertEqual(wizard.scans, 2)
        XCTAssertEqual(wizard.again, 7)
        XCTAssertEqual(wizard.adopt?.url, "rtsp://192.168.8.10:554/stream1")
        let port = try XCTUnwrap(wizard.todo.first)
        XCTAssertEqual(port.options.first, WizardChip(cidr: "192.168.144.2/24", text: "192.168.144.2/24 · found 192.168.144.25"))
        XCTAssertTrue(port.busy)
        XCTAssertEqual(wizard.rest.first?.detail, "this is the device's own uplink")
        XCTAssertEqual(wizard.templates.first, WizardMake(id: "siyi", label: "SIYI camera", sub: "ZT30", action: "Set up"))
        XCTAssertEqual(wizard.template, WizardFound(n: 1, refusal: "", camera: nil, name: "siyi"))
        XCTAssertEqual(wizard.apply, WizardApply(n: 0, state: "failed", text: "That name is taken", field: "name", url: "", camera: ""))
        XCTAssertEqual(wizard.notice.n, 1)
        XCTAssertEqual(parseDeviceWizard(nil).session, -1)
    }

    func testReadsWhatContinueDoesNext() {
        let refused = parseWizardNext(
            JSON.parse(#"{"kind":"refuse","rtsp":"rtsp://192.168.8.10:554/stream","name":null,"camera":"cam","refusal":{"text":"Already used","warn":true,"field":null},"dupAck":"rtsp://192.168.8.10:554/stream","iface":"","ip":""}"#)
        )
        XCTAssertEqual(refused?.refusal, WizardRefusal(text: "Already used", warn: true, field: nil))
        XCTAssertNotNil(refused)
        XCTAssertNil(refused?.name)
        let port = parseWizardNext(JSON.parse(#"{"kind":"port","rtsp":"rtsp://192.168.50.99/s","name":"cam","camera":"cam","refusal":null,"dupAck":"","iface":"eth0","ip":"192.168.50.2/24"}"#))
        XCTAssertEqual(port, WizardNext(kind: "port", rtsp: "rtsp://192.168.50.99/s", name: "cam", camera: "cam", refusal: nil, dupAck: "", iface: "eth0", ip: "192.168.50.2/24"))
        XCTAssertEqual(parseWizardForm(JSON.parse(#"{"next":"Add camera","skipsPort":true,"applyNote":""}"#)), WizardForm(next: "Add camera", skipsPort: true, applyNote: ""))
        XCTAssertEqual(parseWizardForm(nil), NO_FORM)
    }

    func testScanLeadsUntilAnAddressIsFilledThenContinueTakesOver() {
        XCTAssertTrue(scanLeads("", false))
        XCTAssertFalse(continueLeads(""))
        XCTAssertFalse(scanLeads("rtsp://192.168.8.99:554/stream", false))
        XCTAssertTrue(continueLeads("rtsp://192.168.8.99:554/stream"))
        XCTAssertFalse(scanLeads("", true))
    }
}
