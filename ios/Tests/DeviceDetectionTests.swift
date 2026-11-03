import XCTest
@testable import Aircast

final class DeviceDetectionTests: XCTestCase {
    private let served = JSON.parse("""
        {
          "status": {"level": 3, "text": "sim · Sim targets · on the CPU"},
          "unavailable": true,
          "cameras": [
            {"name": "front", "on": true, "status": {"level": 0, "text": "live"}},
            {"name": "belly", "on": false, "status": {"level": 3, "text": "off"}}
          ],
          "models": [
            {"name": "sim", "path": "/m/sim.onnx", "about": "Sim targets", "size": "12 MB", "classes": {"level": 3, "text": "3 classes"}, "engine": "CPU", "active": true},
            {"name": "visdrone", "path": "/m/visdrone.hef", "size": "9 MB", "classes": {"level": 1, "text": "no labels"}, "engine": "Hailo-8", "active": false}
          ]
        }
        """)

    func testReadsTheDetectionPageTheDeviceServes() throws {
        let view = try XCTUnwrap(parseDetection(served))
        XCTAssertEqual(view.status, DeviceBadge(level: 3, text: "sim · Sim targets · on the CPU"))
        XCTAssertTrue(view.unavailable)
        XCTAssertEqual(view.cameras, [
            DetectCamera(name: "front", on: true, status: DeviceBadge(level: 0, text: "live")),
            DetectCamera(name: "belly", on: false, status: DeviceBadge(level: 3, text: "off")),
        ])
        XCTAssertEqual(view.models.first, DetectModel(name: "sim", path: "/m/sim.onnx", about: "Sim targets", size: "12 MB", classes: DeviceBadge(level: 3, text: "3 classes"), engine: "CPU", active: true))
        XCTAssertEqual(view.models.last, DetectModel(name: "visdrone", path: "/m/visdrone.hef", about: "", size: "9 MB", classes: DeviceBadge(level: 1, text: NO_LABELS), engine: "Hailo-8", active: false))
        XCTAssertTrue(detecting(view))
    }

    func testADeviceWithNoCameraDetectingSaysSoAndAnAnswerWithoutAStatusIsNotThisPage() throws {
        var idle = served.object ?? [:]
        idle["cameras"] = JSON.parse(#"[{"name": "front", "on": false, "status": {"level": 3, "text": "off"}}]"#)
        XCTAssertFalse(detecting(try XCTUnwrap(parseDetection(.object(idle)))))
        XCTAssertNil(parseDetection(JSON.parse("{}")))
    }

    func testAnUploadSendsTheModelAndOnlyTheExtrasThatWereGiven() {
        let hef = URL(fileURLWithPath: "/tmp/m.hef")
        let labels = URL(fileURLWithPath: "/tmp/m.labels")
        XCTAssertEqual(modelFields(ModelUpload(hef: hef, labels: nil, about: "  ", use: false)), [DevicePart(name: "hef", file: hef)])
        XCTAssertEqual(
            modelFields(ModelUpload(hef: hef, labels: labels, about: " Aerial ", use: true)),
            [DevicePart(name: "hef", file: hef), DevicePart(name: "labels", file: labels), DevicePart(name: "about", value: "Aerial"), DevicePart(name: "use", value: "1")]
        )
    }

    func testAnUploadSaysWhetherItSwitchedAndWhetherAnyCameraIsDetectingOnIt() {
        XCTAssertEqual(modelUploadMessage(true, true, true), ModelUploadOutcome(refusal: nil, message: "Model uploaded and switched — detection restarts on it."))
        XCTAssertEqual(
            modelUploadMessage(true, true, false),
            ModelUploadOutcome(refusal: nil, message: "Model uploaded and switched — no camera is detecting yet. Turn it on for a camera on this page.")
        )
        XCTAssertEqual(
            modelUploadMessage(false, true, true),
            ModelUploadOutcome(refusal: DeviceRefusal(message: "Model uploaded, but switching to it failed — pick it from the list.", field: ""), message: "")
        )
        XCTAssertEqual(modelUploadMessage(false, false, true), ModelUploadOutcome(refusal: nil, message: "Model uploaded — use it from the list when you want it."))
    }

    func testAModelNameIsEscapedIntoItsPathAndItsLabelsPath() {
        XCTAssertEqual(modelPath("aerial v2"), "/api/detect/models/aerial%20v2")
        XCTAssertEqual(labelsPath("aerial v2"), "/api/detect/models/aerial%20v2/labels")
    }

    func testAnUploadPartNamesItsFileOrCarriesItsValue() {
        let hef = URL(fileURLWithPath: "/tmp/m.hef")
        XCTAssertEqual(devicePartJson(DevicePart(name: "hef", file: hef)), JSON.parse(#"{"name":"hef","file":"/tmp/m.hef"}"#))
        XCTAssertEqual(devicePartJson(DevicePart(name: "use", value: "1")), JSON.parse(#"{"name":"use","value":"1"}"#))
    }
}
