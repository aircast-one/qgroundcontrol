import XCTest
@testable import Aircast

final class DetectionsTests: XCTestCase {
    private let served = """
        {"kind":"object","class":"Detections","available":true,"host":"10.0.0.4","camera":"front",
         "stale":false,"ageMs":40,
         "boxes":[{"x":0.1,"y":0.2,"w":0.3,"h":0.4,"label":"car","confidence":0.91,"target":true},
                  {"x":0.5,"y":0.5,"w":0.1,"h":0.1,"label":"person","confidence":0.42,"target":false}],
         "track":null,"error":null}
        """

    private func with(_ changes: [String: JSON]) -> JSON {
        JSON.object((JSON.parse(served).object ?? [:]).merging(changes) { _, new in new })
    }

    func testAFrameCarriesItsBoxesWithTheTargetMarked() {
        let reading = detections(JSON.parse(served))!
        XCTAssertEqual(reading.boxes.map(\.label), ["car", "person"])
        XCTAssertEqual(reading.boxes.map(\.target), [true, false])
        XCTAssertEqual(reading.boxes[0].x, 0.1, accuracy: 1e-9)
        XCTAssertEqual(reading.boxes[0].h, 0.4, accuracy: 1e-9)
        XCTAssertNil(reading.error)
    }

    func testAReplyThatIsNotTheDetectionsViewReadsAsNothing() {
        XCTAssertNil(detections(nil))
        XCTAssertNil(detections(JSON.parse(#"{"kind":"null"}"#)))
    }

    func testNothingIsDrawnWhenTheFeedIsStaleOrUnconfigured() {
        XCTAssertTrue(visibleBoxes(detections(with(["stale": .bool(true)]))).isEmpty)
        XCTAssertTrue(visibleBoxes(detections(with(["available": .bool(false)]))).isEmpty)
        XCTAssertEqual(visibleBoxes(detections(JSON.parse(served))).count, 2)
    }

    func testAConfiguredFeedReportsItsTroubleAndAnUnconfiguredOneStaysQuiet() {
        XCTAssertEqual(detectionTrouble(detections(with(["error": .string("connection refused")]))), "connection refused")
        XCTAssertNil(detectionTrouble(detections(with(["available": .bool(false), "error": .string("connection refused")]))))
        XCTAssertNil(detectionTrouble(detections(JSON.parse(served))))
    }

    func testACaptionNamesWhatWasSeenAndHowSureTheDetectorIs() {
        let reading = detections(JSON.parse(served))!
        XCTAssertEqual(boxCaption(reading.boxes[0]), "car 91%")
        XCTAssertEqual(boxCaption(reading.boxes[1]), "person 42%")
        var blank = reading.boxes[0]
        blank.label = ""
        blank.confidence = 0
        XCTAssertEqual(boxCaption(blank), "")
        var unsure = reading.boxes[0]
        unsure.confidence = 0
        XCTAssertEqual(boxCaption(unsure), "car")
    }
}

final class DetectionTroubleTests: XCTestCase {
    private func reading(_ available: Bool, _ stale: Bool, _ error: String?) -> Detections {
        Detections(available: available, stale: stale, boxes: [], error: error)
    }

    func testAnUnconfiguredDetectorSaysNothing() {
        XCTAssertNil(detectionTrouble(reading(false, true, "boom")))
    }

    func testADetectorThatStoppedSendingFramesSaysSo() {
        XCTAssertEqual(detectionTrouble(reading(true, true, nil)), NO_FRAMES)
    }

    func testAWorkingDetectorWithNothingInViewSaysNothing() {
        XCTAssertNil(detectionTrouble(reading(true, false, nil)))
    }

    func testAReportedErrorOutranksStaleness() {
        XCTAssertEqual(detectionTrouble(reading(true, true, "stream refused")), "stream refused")
    }
}
