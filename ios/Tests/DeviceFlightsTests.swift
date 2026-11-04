import CoreGraphics
import XCTest
@testable import Aircast

final class DeviceFlightsTests: XCTestCase {
    func testReadsTheFlightLogTheDeviceServes() {
        let view = parseFlights(JSON.parse("""
            {"flights":[
            {"id":"f1","title":"Tue 22 Sep, 14:00","summary":"Lasted 22m10s","badge":{"level":3,"text":"3 events"},
             "sections":[{"title":"Sortie 1","span":"14:02:10 – 14:12:40 · 10m30s",
               "videos":[{"camera":"cam2","label":"Video · cam2","url":"/api/recordings/cam2/video?from=a&to=b","share":{"label":"Share version · cam2","confirm":"Send about 1.2 MB?","camera":"cam2","from":"a","to":"b"}}],
               "lines":[{"index":1,"clock":"14:02:14","text":"cam stopped sending video","level":2,"alert":true,"repeats":"repeated ×4 until 14:02:44"}]}],
             "trace":{"step":30,"startUnix":1790085600,"tzOffset":10800,
               "lanes":[{"key":"signal","label":"Modem signal","chip":"Signal","values":[80,41],"notes":["4G","3G"],"unit":"percent","max":100,"default":true,"peak":"41%"}],
               "markers":[{"offset":60,"label":"14:02:05 modem","level":2}],
               "spans":[{"label":"Sortie 1","from":0.1,"to":0.9}]},
             "csv":"/api/flights/f1/samples.csv","json":"/api/flights/f1"},
            {"id":"f2","title":"Wed","badge":{"level":0,"text":"in progress"},"open":true,"sections":[],"dropped":"12 more events happened"}]}
            """))
        let traced = view?.flights[0]
        let live = view?.flights[1]
        XCTAssertEqual(
            traced?.sections.first?.lines.first,
            FlightLine(index: 1, clock: "14:02:14", text: "cam stopped sending video", level: DeviceLevel.Blocked, alert: true, repeats: "repeated ×4 until 14:02:44", heldBack: false)
        )
        XCTAssertEqual(traced?.sections.first?.videos.first?.share?.camera, "cam2")
        XCTAssertEqual(traced?.trace?["step"].int, 30)
        XCTAssertTrue(traced.map(hasLines) ?? false)
        XCTAssertNil(live?.trace)
        XCTAssertEqual(live?.open, true)
        XCTAssertFalse(live.map(hasLines) ?? true)
        XCTAssertNil(parseFlights(JSON.parse("{}")))
        XCTAssertEqual(parseFlights(JSON.parse(#"{"flights":[]}"#))?.flights, [])
    }

    func testACopiedLinkOpensTheFlyAppStraightOnThatFlight() {
        XCTAssertEqual(flightLink("http://falcon.local/fly/", "20260922-140000"), "http://falcon.local/fly/#device=%2Fflights%3Fflight%3D20260922-140000")
    }

    func testAFlightOpensWithItsOwnDefaultsUntilSomeoneChooses() {
        XCTAssertEqual(laneKeys(nil, ["signal", "bitrate"]), ["signal", "bitrate"])
        XCTAssertEqual(laneKeys(["cpu"], ["signal", "bitrate"]), ["cpu"])
    }

    func testChoosingNothingStaysChosen() {
        XCTAssertEqual(laneKeys([], ["signal"]), [])
    }

    func testAChipAddsTheLaneItNamesAndTakesItAwayAgain() {
        XCTAssertEqual(toggled(["signal"], "cpu"), ["signal", "cpu"])
        XCTAssertEqual(toggled(["signal", "cpu"], "signal"), ["cpu"])
    }

    func testDrawsTheChartTheCoreComputedInTheCoresPlotUnits() {
        let chart = parseFlightChart(JSON.parse("""
            {"lanes":[{"key":"signal","label":"Modem signal","chip":"Signal","peak":"41%","default":true,
              "lines":[[[0,5],[25,50.5]],[[75,95],[100,5]]],
              "grid":[{"y":72.5,"label":"25%"}],
              "markers":[{"x":50,"label":"14:02:05 modem","index":1}],
              "readings":["14:00:00 · 80% · 4G","14:00:30 · no reading"],
              "ticks":[{"x":0,"label":"14:00"}],
              "span":"6m of flight"}],
            "spans":[{"label":"Sortie 1","from":0.1,"to":0.9}]}
            """))
        let lane = chart?.lanes.first
        XCTAssertEqual(lane?.lines, [[CGPoint(x: 0, y: 5), CGPoint(x: 25, y: 50.5)], [CGPoint(x: 75, y: 95), CGPoint(x: 100, y: 5)]])
        XCTAssertEqual(lane?.markers.first, ChartMarker(x: 50, label: "14:02:05 modem", level: DeviceLevel.Neutral, index: 1))
        XCTAssertEqual(lane?.grid.first, ChartGrid(y: 72.5, label: "25%"))
        XCTAssertEqual(lane?.readings[1], "14:00:30 · no reading")
        XCTAssertEqual(lane?.isDefault, true)
        XCTAssertEqual(chart?.spans.first, ChartSpan(label: "Sortie 1", from: 0.1, to: 0.9))
        XCTAssertNil(parseFlightChart(nil))
    }

    func testACursorSnapsToTheNearestSampleOfTheLaneItIsUnder() {
        XCTAssertEqual(sampleAt(3, 0.9), FlightChartSample(index: 2, x: 100))
        XCTAssertEqual(sampleAt(5, 0.3), FlightChartSample(index: 1, x: 25))
        XCTAssertEqual(sampleAt(1, 0.7), FlightChartSample(index: 0, x: 0))
        XCTAssertNil(sampleAt(0, 0.5))
    }

    func testADragInsideAZoomNarrowsTheZoomItIsIn() {
        XCTAssertEqual(composeZoom(nil, 0.6, 0.2), Zoom(from: 0.2, to: 0.6))
        XCTAssertEqual(composeZoom(Zoom(from: 0.5, to: 1.0), 0.0, 0.5), Zoom(from: 0.5, to: 0.75))
    }

    func testAShareVersionShowsTheRenderTheCoreIsFollowingForIt() {
        let share = SortieShare(label: "Share version · cam2", confirm: "Send?", camera: "cam2", from: "20260922T140210", to: "20260922T141240")
        XCTAssertEqual(renderPath(share), "view.deviceRender(cam2,20260922T140210,20260922T141240)")
        XCTAssertEqual(parseRender(nil), FlightRender(canStart: true, progress: "", error: "", url: "", expiry: ""))
        XCTAssertEqual(
            parseRender(JSON.parse(#"{"canStart":false,"progress":"","error":"","url":"https://r2/x.mp4","expiry":"Link works until 22 Oct"}"#)),
            FlightRender(canStart: false, progress: "", error: "", url: "https://r2/x.mp4", expiry: "Link works until 22 Oct")
        )
    }
}
