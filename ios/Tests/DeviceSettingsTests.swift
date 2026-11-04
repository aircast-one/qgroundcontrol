import XCTest
@testable import Aircast

final class DeviceSettingsTests: XCTestCase {
    private let served = JSON.parse("""
        {
          "health": {"level": 2, "text": "Mobile data down", "href": "/network#cellular"},
          "notice": {"level": 1, "text": "v0.9.1 is available — install it under Software."},
          "stats": [{"label": "Services", "value": "All running", "hint": "3 supervised units", "level": 0}],
          "services": [{"label": "Video server", "state": "running", "level": 0}],
          "cloud": {"status": {"level": 3, "text": "waiting for sign-in"}, "stage": "signing-in", "userCode": "ABCD-1234", "signInUrl": "https://app.aircast.one/device", "issuer": "https://api.aircast.one"},
          "software": {
            "status": {"level": 1, "text": "Update failed"},
            "summary": "v0.9.0",
            "notices": [{"level": 2, "title": "The update to v0.9.1 did not finish.", "text": "You can try again.", "detailLabel": "What went wrong", "detail": "E: dpkg"}],
            "candidate": {"title": "v0.9.1 is available", "notesUrl": "https://aircast.one/docs/releases#v0.9.1", "install": "Install v0.9.1", "confirm": "Install v0.9.1?", "success": "Installing v0.9.1"},
            "notes": ["Updates come from the Stable channel."],
            "checked": "Checked just now.",
            "canCheck": true
          },
          "hostname": "falcon"
        }
        """)

    func testReadsTheDevicePageTheDeviceServes() {
        let view = parseDeviceSettings(served)
        XCTAssertEqual(view?.health, HealthBadge(level: DeviceLevel.Blocked, text: "Mobile data down", href: "/network#cellular"))
        XCTAssertEqual(view?.notice?.level, DeviceLevel.Warn)
        XCTAssertEqual(view?.stats.first, DeviceStat(label: "Services", value: "All running", hint: "3 supervised units", level: DeviceLevel.Ready))
        XCTAssertEqual(view?.services, [ServiceRow(label: "Video server", state: "running", level: DeviceLevel.Ready)])
        XCTAssertEqual(view?.cloud.stage, CloudStage.SigningIn)
        XCTAssertEqual(view?.cloud.userCode, "ABCD-1234")
        XCTAssertEqual(view?.cloud.deviceId, "")
        XCTAssertEqual(view?.cloud.error, "")
        XCTAssertEqual(view?.software.candidate?.install, "Install v0.9.1")
        XCTAssertEqual(view?.software.notices.first?.detail, "E: dpkg")
        XCTAssertEqual(view?.software.notes, ["Updates come from the Stable channel."])
        XCTAssertEqual(view?.hostname, "falcon")
    }

    func testAnAnswerWithoutAHealthLineIsNotADevicePage() {
        XCTAssertNil(parseDeviceSettings(JSON.parse(#"{"stats":[]}"#)))
    }

    func testMissingPiecesReadAsNothingToShowNeverAsACrash() {
        let view = parseDeviceSettings(JSON.parse(#"{"health":{"level":0,"text":"All systems operational"}}"#))
        XCTAssertEqual(view?.health.href, "")
        XCTAssertNil(view?.notice)
        XCTAssertEqual(view?.cloud.stage, CloudStage.Connect)
        XCTAssertNil(view?.software.candidate)
        XCTAssertFalse(view?.software.canCheck ?? true)
    }
}
