import Foundation
import XCTest
@testable import Aircast

private func withKey(_ json: JSON, _ key: String, _ value: JSON?) -> JSON {
    guard case .object(var map) = json else { return json }
    map[key] = value
    return .object(map)
}

final class DeviceScreenTests: XCTestCase {
    private let served = JSON.parse("""
        {"name":"falcon","level":2,"headline":"Camera is not sending video","detail":"Check the cable.",
         "action":{"label":"Check the camera","href":"/streaming?camera=cam"},
         "checks":[{"label":"Camera","value":"no video","level":2,"href":"/streaming?camera=cam"}],
         "pages":[{"title":"Ground station links","href":"/connect","icon":"qr-code"},{"title":"Cameras","href":"/streaming","icon":"video","summary":"1 camera, 1 dark","needs":true}]}
        """)

    private var aircraft: JSON {
        var json = withKey(served, "setup", JSON.parse(#"{"label":"Finish setting up","href":"/setup"}"#))
        json = withKey(json, "checks", JSON.parse("""
            [{"label":"Camera","value":"no video","level":2,"href":"/streaming?camera=cam"},{"label":"Telemetry","value":"waiting for autopilot","level":1,"href":"/telemetry"},{"label":"Remote access","value":"not set up","level":1,"href":"/network#tailscale"}]
            """))
        return withKey(json, "pages", JSON.parse("""
            [{"title":"Ground station links","href":"/connect","icon":"qr-code"},{"title":"Cameras","href":"/streaming","icon":"video"},
             {"title":"Video engine","href":"/settings/video","icon":"sliders-horizontal"},{"title":"Detection","href":"/settings/detection","icon":"crosshair"},
             {"title":"Connection","href":"/network","icon":"network"},
             {"title":"Telemetry","href":"/telemetry","icon":"activity"},{"title":"Recordings","href":"/settings/recordings","icon":"gauge"},
             {"title":"Flight log","href":"/flights","icon":"clock"},{"title":"Device","href":"/settings/device","icon":"settings"},
             {"title":"Engineering","href":"/settings/engineering","icon":"server"},{"title":"Watching & recording","href":"/settings/camera?camera=cam","icon":"sliders-horizontal"},
             {"title":"Set up this device step by step","href":"/setup","icon":"scan-search"}]
            """))
    }

    func testReadsWhatTheDeviceSaysAboutItself() throws {
        let status = try XCTUnwrap(parseDeviceStatus(served))
        XCTAssertEqual(status.level, DeviceLevel.Blocked)
        XCTAssertEqual(status.action, DeviceAction(label: "Check the camera", href: "/streaming?camera=cam"))
        XCTAssertEqual(status.checks.first, DeviceCheck(label: "Camera", value: "no video", level: 2, href: "/streaming?camera=cam", action: ""))
        XCTAssertEqual(status.pages.map(\.needs), [false, true])
        XCTAssertTrue(deviceNeedsAttention(status))
        XCTAssertEqual(chipText(status), "falcon · Camera is not sending video")
        XCTAssertFalse(deviceNeedsAttention(parseDeviceStatus(withKey(withKey(served, "level", .number(0)), "action", nil))))
        XCTAssertNil(try XCTUnwrap(parseDeviceStatus(withKey(served, "action", nil))).action)
        XCTAssertNil(status.setup)
        XCTAssertEqual(try XCTUnwrap(parseDeviceStatus(aircraft)).setup, DeviceAction(label: "Finish setting up", href: "/setup"))
        XCTAssertNil(parseDeviceStatus(JSON.parse(#"{"name":"falcon"}"#)))
    }

    func testEveryDevicePageHasItsOwnIconAndAnUnknownOneFallsBack() {
        XCTAssertEqual(devicePageIcon("video"), .videocam)
        XCTAssertEqual(devicePageIcon("scan-search"), .build)
        XCTAssertEqual(devicePageIcon("crosshair"), .centerFocus)
        XCTAssertEqual(devicePageIcon("unheard-of"), .settings)
    }

    func testEveryPageTheDeviceLinksToAndEveryPageAScreenOpensIsDrawnNatively() throws {
        let native = { (href: String) in DEVICE_SCREENS[devicePath(href).path] != nil }
        let status = try XCTUnwrap(parseDeviceStatus(aircraft))
        let linked = ([status.action?.href, status.setup?.href].compactMap { $0 } + status.checks.map(\.href) + status.pages.map(\.href)).filter { !$0.isEmpty }
        XCTAssertEqual(linked.count, 17)
        XCTAssertEqual(linked.filter { !native($0) }, [])
        let sources = URL(fileURLWithPath: #filePath).deletingLastPathComponent().deletingLastPathComponent().appendingPathComponent("Sources/UI")
        let screens = try FileManager.default.contentsOfDirectory(atPath: sources.path)
            .filter { $0.hasPrefix("Device") || $0 == "CameraWizard.swift" }
        let pattern = try NSRegularExpression(pattern: #"onOpen(?:From)?\("([^"]+)"[,)]"#)
        let opened = try screens.flatMap { name -> [String] in
            let text = try String(contentsOf: sources.appendingPathComponent(name), encoding: .utf8)
            return pattern.matches(in: text, range: NSRange(text.startIndex..., in: text)).compactMap { match in
                Range(match.range(at: 1), in: text).map { String(text[$0]) }
            }
        }
        XCTAssertGreaterThan(opened.count, 5)
        XCTAssertEqual(opened.filter { !native($0) }, [])
    }

    func testADeviceHrefSplitsIntoTheScreenItOpensAndWhatItAsksThatScreenFor() {
        XCTAssertEqual(devicePath("/settings/camera?camera=front%20cam"), DevicePath(path: "/settings/camera", query: ["camera": "front cam"]))
        XCTAssertEqual(devicePath("/network#tailscale"), DevicePath(path: "/network", query: [:]))
    }

    func testOpeningFromAnotherScreenReturnsThere() {
        XCTAssertEqual(openedFrom(["/connect", "/streaming"], "/settings/camera?camera=cam", "/streaming?camera=cam"), ["/connect", "/streaming?camera=cam", "/settings/camera?camera=cam"])
    }

    func testTheDeviceTabIsAlwaysThereAndARequestedDevicePageOpensOnIt() {
        XCTAssertEqual(SettingsGroup.allCases.last, .Device)
        XCTAssertEqual(openingGroup(DEVICE_PAGE), .Device)
        XCTAssertTrue(pageLook(DEVICE_PAGE).inline)
    }

    func testTheDeviceHomeCarriesTheLinkAndTheHealthTheFlightSheetShowed() throws {
        var json = withKey(served, "link", JSON.parse("""
            {"carrier":"Wi-Fi (wlan0)","rate":"↓ 1 kB/s","standby":"Magti · lte","charts":[{"label":"Wi-Fi","values":[60,64],"max":100,"value":"64%","level":3}]}
            """))
        json = withKey(json, "health", JSON.parse("""
            {"memory":"400 MB of 1 GB","warning":{"level":1,"text":"Undervoltage seen since start"},"charts":[{"label":"CPU","values":[10,92],"max":100,"value":"92%","level":2}]}
            """))
        json = withKey(json, "events", .string("2 events this flight"))
        let status = try XCTUnwrap(parseDeviceStatus(json))
        XCTAssertEqual(status.link?.carrier, "Wi-Fi (wlan0)")
        XCTAssertEqual(status.link?.charts.first, DeviceChartView(label: "Wi-Fi", values: [60, 64], max: 100, value: "64%", level: 3))
        XCTAssertEqual(status.health?.warning, DeviceBadge(level: 1, text: "Undervoltage seen since start"))
        XCTAssertEqual(status.health?.charts.first?.level, DeviceLevel.Blocked)
        XCTAssertEqual(status.events, "2 events this flight")
        XCTAssertNil(try XCTUnwrap(parseDeviceStatus(served)).link)
    }
}

final class DeviceViewTests: XCTestCase {
    func testADeviceUrlIsThePathItselfUntilADeviceIsConnectedAndOnTheDeviceAfter() {
        XCTAssertEqual(resolveDeviceUrl("", "/api/views/aircraft"), "/api/views/aircraft")
        XCTAssertEqual(resolveDeviceUrl("http://192.168.1.100", "/api/views/aircraft"), "http://192.168.1.100/api/views/aircraft")
        XCTAssertEqual(resolveDeviceUrl("http://falcon.local:8080", "/api/logs"), "http://falcon.local:8080/api/logs")
    }

    func testTheCoresReadingSaysWhichDeviceAnsweredAndCarriesItsView() {
        let reading = deviceReading(JSON.parse(#"{"origin":"http://10.0.0.5","host":"10.0.0.5","answered":true,"view":{"headline":"Ready"}}"#))
        XCTAssertEqual(reading, DeviceReading(origin: "http://10.0.0.5", host: "10.0.0.5", answered: true, view: reading.view))
        XCTAssertEqual(reading.view?["headline"].string, "Ready")
        XCTAssertEqual(deviceReading(nil), DeviceReading(origin: "", host: "", answered: false, view: nil))
    }

    func testARefusalNamesTheFieldTheDeviceBlamedAndNoAnswerSaysSo() {
        XCTAssertEqual(
            deviceRefusal(JSON.parse(#"{"ok":false,"reason":"baud must be a number","field":"baud"}"#)),
            DeviceRefusal(message: "baud must be a number", field: "baud")
        )
        XCTAssertNil(deviceRefusal(JSON.parse(#"{"ok":true}"#)))
        XCTAssertEqual(deviceRefusal(nil), DEVICE_NO_ANSWER)
        XCTAssertEqual(deviceAnswer(JSON.parse(#"{"ok":true,"result":null}"#)), DeviceAnswer(refusal: nil, result: nil))
    }

    func testAChartSpansItsWidthAndClampsValuesToItsScale() {
        let points = chartPoints([0, 50, 150], 100, 200, 40)
        XCTAssertEqual(points.map(\.x), [0, 100, 200])
        XCTAssertEqual(points.map(\.y), [40, 20, 0])
        XCTAssertEqual(chartPoints([5], 100, 200, 40), [])
    }

    func testAFormSendsWhatWasTypedOverWhatTheDeviceShowedAndShowsOnlyTheFieldsThatApply() {
        let fields = [
            DeviceField(name: "method", label: "Addressing", value: "auto"),
            DeviceField(name: "address", label: "Address", value: "", shownWhen: { $0["method"] == "manual" }),
        ]
        XCTAssertEqual(formBody(fields, ["method": "manual"]), ["method": "manual", "address": ""])
        XCTAssertEqual(shownFields(fields, formBody(fields, [:])).map(\.name), ["method"])
        XCTAssertEqual(shownFields(fields, formBody(fields, ["method": "manual"])).map(\.name), ["method", "address"])
    }

    func testAPathPieceIsEscapedTheWayTheDeviceDecodesIt() {
        XCTAssertEqual(uriComponent("aerial v2"), "aerial%20v2")
        XCTAssertEqual(uriComponent("2026-10-18T10:00:00+04:00"), "2026-10-18T10%3A00%3A00%2B04%3A00")
    }
}
