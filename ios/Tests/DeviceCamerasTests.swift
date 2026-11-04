import XCTest
@testable import Aircast

final class DeviceCamerasTests: XCTestCase {
    private let wizard = #"{"name":"","new":true,"suggestedName":"cam","interfaces":[],"transports":[],"others":[],"templates":[],"subnets":[]}"#

    private func camera(_ name: String, live: Bool) -> String {
        let trouble = live ? "" : #""diagnosis": {"level": 2, "text": "Camera not responding"}, "fix": {"label": "Scan for it", "scan": true},"#
        return """
            {
              "name": "\(name)",
              "status": {"level": \(live ? 0 : 2), "text": "\(live ? "live" : "unreachable")"},
              "live": \(live),
              "preview": "\(live ? "/whep/\(name)" : "")",
              "placeholder": "\(live ? "" : "Camera offline")",
              "watch": "/watch/\(name)",
              "copies": [{"label": "Copy watch link", "text": "http://dev/watch/\(name)", "qr": false}],
              "facts": [{"label": "Source", "value": "rtsp://10.0.0.1/s"}],
              "bitrate": {"label": "Bitrate", "values": [1, 2], "max": 2, "value": "16 bps", "level": 3},
              "links": [{"proto": "RTSP", "label": "QGroundControl", "url": "rtsp://dev:8554/\(name)"}],
              "chips": [{"key": "cloud", "label": "cloud connected", "level": 0, "href": "/settings/camera?camera=\(name)#cam-watch"}],
              \(trouble)
              "wizard": {"name": "\(name)", "new": false, "suggestedName": "cam", "interfaces": [], "transports": [], "others": [], "templates": [], "subnets": []},
              "setup": {
                "watch": {
                  "part": {"title": "Where to watch", "summary": "Ground station, browser, QR codes.", "badge": {"level": 1, "text": "cloud sign-in"}, "open": true},
                  "cloud": true,
                  "signIn": true,
                  "cloudLinks": [],
                  "links": []
                },
                "radio": {
                  "part": {"title": "Video without cellular", "summary": "s"},
                  "notice": {"level": 3, "text": "needs the engine", "link": {"label": "What carries video", "href": "/settings/video"}},
                  "form": false
                },
                "servers": {
                  "part": {"title": "Your own server", "summary": "s"},
                  "dests": [{"kind": "cloudflare", "part": {"title": "Cloudflare Realtime", "summary": "s"}, "on": true, "app": "a", "track": "t", "watch": "/watch/\(name)?via=cloudflare"}]
                },
                "shares": {
                  "part": {"title": "Share links", "summary": "s"},
                  "links": [{"id": "x", "label": "—", "url": "http://dev/s/x", "dead": "Turned off 1 Jan 09:00"}],
                  "hours": [{"value": "24", "label": "1 day"}]
                },
                "detect": {"part": {"title": "Detection overlay", "summary": "s"}, "on": false},
                "record": {"part": {"title": "Recording", "summary": "s"}, "on": true, "full": true}
              },
              "topic": {"title": "Watching & recording", "href": "/settings/camera?camera=\(name)", "icon": "sliders-horizontal", "summary": "where it goes"}
            }
            """
    }

    private var served: JSON {
        JSON.parse("""
            {
              "cameras": [\(camera("cam2", live: true)), \(camera("cam3", live: false))],
              "add": \(wizard),
              "cloudLink": "https://app/x",
              "video": {"title": "Video engine", "href": "/settings/video", "icon": "sliders-horizontal"}
            }
            """)
    }

    private func cameras() -> [CameraView] { parseCameras(served)?.cameras ?? [] }

    func testReadsTheCamerasViewTheDeviceServes() throws {
        let view = try XCTUnwrap(parseCameras(served))
        XCTAssertEqual(view.cameras.map(\.name), ["cam2", "cam3"])
        XCTAssertEqual(view.cloudLink, "https://app/x")
        XCTAssertTrue(view.add.isNew)
        let live = view.cameras[0]
        let dark = view.cameras[1]
        XCTAssertEqual(live.status, DeviceBadge(level: DeviceLevel.Ready, text: "live"))
        XCTAssertEqual(live.preview, "/whep/cam2")
        XCTAssertEqual(live.bitrate?.max, 2)
        XCTAssertNil(live.diagnosis)
        XCTAssertEqual(dark.fix, CameraFix(label: "Scan for it", scan: true))
        XCTAssertEqual(dark.diagnosis?.level, DeviceLevel.Blocked)
        XCTAssertEqual(live.setup.watch.part.badge, DeviceBadge(level: DeviceLevel.Warn, text: "cloud sign-in"))
        XCTAssertEqual(live.setup.radio.notice.link, NoticeLink(label: "What carries video", href: "/settings/video"))
        let dest = try XCTUnwrap(live.setup.servers.dests.first)
        XCTAssertEqual([dest.kind, dest.app, dest.track, dest.watch], ["cloudflare", "a", "t", "/watch/cam2?via=cloudflare"])
        XCTAssertTrue(dest.on)
        XCTAssertEqual(live.setup.shares.links.first?.dead, "Turned off 1 Jan 09:00")
        XCTAssertNil(live.setup.detect.notice)
        XCTAssertEqual([live.setup.record.on, live.setup.record.full], [true, true])
        XCTAssertEqual(live.setup.record.usage, "")
        XCTAssertNil(live.setup.gimbal)
        XCTAssertEqual(live.topic.href, "/settings/camera?camera=cam2")
        XCTAssertNil(parseCameras(JSON.parse(#"{"cameras": []}"#)))
    }

    func testOpensTheOnlyCameraWallsSeveralAndAddsWhenAskedOrWhenThereAreNone() {
        let all = cameras()
        guard all.count == 2 else { return XCTFail("expected two cameras, read \(all.count)") }
        let one = Array(all.prefix(1))
        XCTAssertEqual(camerasShown(one, "", false), .Camera(all[0]))
        XCTAssertEqual(camerasShown(all, "", false), .Wall)
        XCTAssertEqual(camerasShown(all, "cam3", false), .Camera(all[1]))
        XCTAssertEqual(camerasShown(all, "gone", false), .Wall)
        XCTAssertEqual(camerasShown(all, "cam3", true), .Add)
        XCTAssertEqual(camerasShown([], "cam2", false), .Add)
    }

    func testAnnouncesACameraOnlyWhenItComesBackFromDark() {
        let before = liveness(cameras())
        XCTAssertEqual(before["cam3"], false)
        let back = cameras().map { camera -> CameraView in
            var lit = camera
            lit.live = true
            return lit
        }
        XCTAssertEqual(cameBack(before, back), ["cam3"])
        XCTAssertEqual(cameBack([:], back), [])
        XCTAssertEqual(cameBack(liveness(back), back), [])
    }

    func testComposesTheCloudflareTargetFromTheAppAndTrack() {
        XCTAssertEqual(cloudflareTarget(" app-123 ", "cam2-track"), "cloudflare://app-123/cam2-track")
        XCTAssertEqual(cloudflareTarget("app-123", ""), "")
    }

    func testAChipOpensTheSettingItNames() {
        XCTAssertEqual(openedSection("/settings/camera?camera=cam2#cam-watch"), "cam-watch")
        XCTAssertEqual(openedSection("/settings/camera?camera=cam2"), "")
    }

    func testALinkFollowedFromACamerasDetectionSectionComesBackToThatSection() {
        let back = cameraSectionHref("front cam", CameraSetupSection.Detect)
        XCTAssertEqual(openedFrom(["/streaming", "/settings/camera?camera=front%20cam"], "/settings/detection", back), ["/streaming", back, "/settings/detection"])
        XCTAssertEqual(devicePath(back), DevicePath(path: "/settings/camera", query: ["camera": "front cam"]))
        XCTAssertEqual(openedSection(back), CameraSetupSection.Detect)
    }

    func testAPickedVideoKeepsItsNameAndFallsBackToVideoMp4() {
        XCTAssertEqual(stagedVideoName("flight/one.mp4"), "flight_one.mp4")
        XCTAssertEqual(stagedVideoName(""), "video.mp4")
    }
}
