import XCTest
@testable import Aircast

final class DeviceSetupTests: XCTestCase {
    private let served = JSON.parse("""
        {
          "start": 3,
          "steps": [
            {"key": "network", "label": "Internet", "title": "Connect to the internet", "lead": "This device needs internet.", "status": {"level": 0, "text": "ready"}},
            {"key": "tailscale", "label": "Remote access", "title": "Remote access", "lead": "Reach it from anywhere."},
            {"key": "camera", "label": "Camera", "title": "Camera", "lead": "Point it at the camera."},
            {"key": "telemetry", "label": "Telemetry", "title": "Telemetry", "lead": "Plug the flight controller in."},
            {"key": "done", "label": "Done", "title": "Almost there", "lead": "One item still needs attention."}
          ],
          "internet": {
            "online": {"level": 0, "text": "Online — this device can reach the internet."},
            "joinOpen": false,
            "reopen": {"host": "falcon.local", "hostname": "falcon"},
            "connection": {"title": "Connection", "href": "/network", "icon": "network", "summary": "home-wifi"},
            "hint": "You can continue either way."
          },
          "remote": {"status": {"level": 2, "text": "offline"}, "summary": "no remote access", "facts": [], "addresses": [], "cancel": false, "connect": "Connect to Tailscale"},
          "camera": {
            "problem": "Camera offline — the source is saved but no video is arriving.",
            "wizard": {"name": "cam", "new": false, "source": "rtsp://cam", "interfaces": [], "transports": [], "others": [], "templates": [], "subnets": []},
            "cameras": {"title": "Cameras", "href": "/streaming", "icon": "video", "summary": "1 camera, 1 dark", "needs": true},
            "hint": "The rest lives on Cameras."
          },
          "telemetry": {
            "waiting": {"level": 3, "text": "Waiting for a flight controller on USB."},
            "endpoints": [{"proto": "udps", "label": "MAVLink", "url": "falcon.tail.ts.net:14550"}],
            "hint": "Point your ground station at this UDP address."
          },
          "checks": [
            {"label": "Internet", "status": {"level": 0, "text": "ready"}, "step": 1},
            {"label": "Camera stream", "status": {"level": 3, "text": "not set up"}, "step": 3}
          ]
        }
        """)

    private func servedWith(_ key: String, _ value: JSON) -> JSON {
        var map = served.object ?? [:]
        map[key] = value
        return .object(map)
    }

    func testReadsTheWalkthroughTheDeviceServes() {
        let view = parseDeviceSetup(served)
        XCTAssertEqual(view?.start, 3)
        XCTAssertEqual(view?.steps.map(\.key), ["network", "tailscale", "camera", "telemetry", "done"])
        XCTAssertEqual(view?.steps[0].status, DeviceBadge(level: DeviceLevel.Ready, text: "ready"))
        XCTAssertNil(view?.steps[1].status)
        XCTAssertEqual(view?.internet.reopen, WifiReopen(host: "falcon.local", hostname: "falcon"))
        XCTAssertEqual(view?.internet.connection.href, "/network")
        XCTAssertEqual(view?.remote.connect, "Connect to Tailscale")
        XCTAssertEqual(view?.camera.wizard.isNew, false)
        XCTAssertEqual(view?.camera.watch, "")
        XCTAssertEqual(view?.camera.cameras.needs, true)
        XCTAssertTrue((view?.telemetry.waiting?.text ?? "").contains("Waiting for a flight controller"))
        XCTAssertEqual(view?.telemetry.endpoints.first?.url, "falcon.tail.ts.net:14550")
        XCTAssertEqual(view?.checks[1], SetupCheckRow(label: "Camera stream", status: DeviceBadge(level: DeviceLevel.Neutral, text: "not set up"), step: 3))
    }

    func testRefusesABodyThatIsNotAWalkthroughAndKeepsAStrayStartOnTheTrail() {
        XCTAssertNil(parseDeviceSetup(nil))
        XCTAssertNil(parseDeviceSetup(servedWith("steps", .array([]))))
        XCTAssertNil(parseDeviceSetup(servedWith("camera", .object([:]))))
        XCTAssertEqual(parseDeviceSetup(servedWith("start", .number(9)))?.start, 5)
        XCTAssertEqual(parseDeviceSetup(servedWith("start", .number(0)))?.start, 1)
        XCTAssertEqual(clampStep(6, 5), 5)
    }

    func testTheFooterOffersWhatEachStepAllows() {
        XCTAssertEqual(setupNav(1, 5, false), SetupNav(back: false, later: true, next: .Next, finish: false))
        XCTAssertEqual(setupNav(3, 5, false), SetupNav(back: true, later: true, next: .Next, finish: false))
        XCTAssertEqual(setupNav(5, 5, false), SetupNav(back: true, later: false, next: nil, finish: true))
    }

    func testACameraBeingEditedHoldsTheWizardUntilItIsAppliedOrSkipped() {
        XCTAssertTrue(cameraHoldsTheWizard("camera", true, 1))
        XCTAssertTrue(cameraHoldsTheWizard("camera", true, 2))
        XCTAssertFalse(cameraHoldsTheWizard("camera", true, 3))
        XCTAssertFalse(cameraHoldsTheWizard("camera", false, 1))
        XCTAssertFalse(cameraHoldsTheWizard("telemetry", true, 1))
        XCTAssertEqual(setupNav(3, 5, true), SetupNav(back: false, later: true, next: .Skip, finish: false))
    }
}
