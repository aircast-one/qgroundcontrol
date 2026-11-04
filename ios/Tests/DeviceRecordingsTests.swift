import XCTest
@testable import Aircast

final class DeviceRecordingsTests: XCTestCase {
    private let segment = RecordingSegment(at: "2026-10-18T10:00:00+04:00", started: "18 Oct 10:00:00", duration: "10 min", size: "1.0 MB", url: "/v", open: false, noTelemetry: false)

    func testReadsTheRecordingsPageTheDeviceServes() {
        let view = parseRecordings(JSON.parse("""
            {
              "limit": {"value": "8589934592", "options": [{"value": "2147483648", "label": "2 GB"}], "used": "2.0 KB used"},
              "cameras": [{"name": "cam2", "recording": true, "note": "Recording · 2.0 KB on disk"}],
              "recorded": [{"name": "cam2", "status": {"level": 1, "text": "storage full"}, "summary": "1 recording", "full": true, "usage": "2.0 KB on this camera", "segments": [{"at": "a", "started": "s", "size": "2.0 KB", "url": "/v", "open": true}]}]
            }
            """))
        XCTAssertEqual(view?.limit.options, [DeviceOption(value: "2147483648", label: "2 GB")])
        XCTAssertEqual(view?.cameras.first, RecordCamera(name: "cam2", recording: true, note: "Recording · 2.0 KB on disk"))
        XCTAssertEqual(view?.recorded.first?.full, true)
        XCTAssertEqual(view?.recorded.first?.status, DeviceBadge(level: 1, text: "storage full"))
        XCTAssertEqual(view?.recorded.first?.segments.first, RecordingSegment(at: "a", started: "s", duration: "", size: "2.0 KB", url: "/v", open: true, noTelemetry: false))
        XCTAssertNil(parseRecordings(JSON.parse(#"{"cameras":[]}"#)))
    }

    func testOnlyASmallerLimitAsksBeforeItIsSaved() {
        XCTAssertTrue(lowersLimit("8589934592", "2147483648"))
        XCTAssertFalse(lowersLimit("8589934592", "17179869184"))
        XCTAssertFalse(lowersLimit("", "2147483648"))
    }

    func testASegmentIsDeletedByItsStartEscaped() {
        XCTAssertEqual(segmentPath("cam2", segment), "/api/recordings/cam2?start=2026-10-18T10%3A00%3A00%2B04%3A00")
        XCTAssertEqual(recordPath("cam2"), "/api/streams/cam2/record")
    }

    func testASegmentSaysWhatItIsMissingAndWhenItCanBeHad() {
        XCTAssertEqual(segmentDetail(segment), ["10 min", "1.0 MB"])
        XCTAssertEqual(
            segmentDetail(withChanges(segment) {
                $0.duration = ""
                $0.noTelemetry = true
                $0.open = true
            }),
            ["1.0 MB", "no flight data", "downloadable once it closes"]
        )
    }
}
