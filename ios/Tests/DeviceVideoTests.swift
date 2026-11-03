import XCTest
@testable import Aircast

final class DeviceVideoTests: XCTestCase {
    private let served = JSON.parse("""
        {
          "radio": {"status": {"level": 0, "text": "on air"}, "channel": "165", "region": "BO", "txPower": "", "details": "UDP 5602"},
          "sharing": {"status": {"level": 1, "text": "finish setup"}, "stage": "deploy", "deployUrl": "https://deploy.workers.cloudflare.com/?url=x", "signKey": "sk", "deviceToken": "dt"},
          "engineering": {"title": "Engineering", "href": "/settings/engineering", "icon": "server", "summary": "mediamtx"}
        }
        """)

    func testReadsTheVideoPageTheDeviceServes() {
        let view = parseVideo(served)
        XCTAssertEqual(view?.radio, RadioSettings(status: DeviceBadge(level: 0, text: "on air"), channel: "165", region: "BO", txPower: "", details: "UDP 5602"))
        XCTAssertEqual(view?.sharing.stage, SharingStage.Deploy)
        XCTAssertEqual(view?.sharing.signKey, "sk")
        XCTAssertEqual(view?.sharing.deviceToken, "dt")
        XCTAssertEqual(view?.sharing.baseUrl, "")
        XCTAssertEqual(view?.engineering, DevicePageLink(title: "Engineering", href: "/settings/engineering", summary: "mediamtx"))
    }

    func testADeviceWithNoRadioHasNoRadioCard() {
        var withoutRadio = served.object ?? [:]
        withoutRadio["radio"] = nil
        XCTAssertNotNil(parseVideo(.object(withoutRadio)))
        XCTAssertNil(parseVideo(.object(withoutRadio))?.radio)
        XCTAssertNil(parseVideo(.object(["radio": served["radio"]])))
    }
}
