import SwiftUI
import XCTest
@testable import Aircast

final class InspectorChartsTests: XCTestCase {
    private let view = JSON.parse(#"""
        {"timeScales":["5 Sec","10 Sec"],"ranges":["Auto","1"],"selectedCharted":[{"field":"roll","chart":0}],
         "charts":[{"rangeX":0,"rangeY":0,"windowMs":5000,"yMin":-1,"yMax":1,"room":true,"plots":[{"label":"ATTITUDE.roll","field":"roll","colour":0,"points":[[2000,0.5]]}]},
                   {"rangeX":1,"rangeY":1,"windowMs":10000,"yMin":null,"yMax":null,"room":false,"plots":[]}]}
        """#)

    func testChartsParseWithTheirPlotsAndWhichFieldsAreCharted() throws {
        let charts = try XCTUnwrap(inspectorCharts(view))
        XCTAssertEqual(charts.timeScales, ["5 Sec", "10 Sec"])
        XCTAssertEqual(charts.charts[0].plots[0].points, [ChartSample(ageMs: 2000, value: 0.5)])
        XCTAssertEqual(charts.charted, ["roll": 0])
        XCTAssertNil(charts.charts[1].yMin)
    }

    func testAFieldChartsOnceTextNeverAndAFullChartTakesNoMore() {
        let charts = inspectorCharts(view)
        XCTAssertTrue(chartToggleEnabled(charts, "roll", "float", 0))
        XCTAssertFalse(chartToggleEnabled(charts, "roll", "float", 1))
        XCTAssertFalse(chartToggleEnabled(charts, "text", "char[50]", 0))
        XCTAssertTrue(chartToggleEnabled(charts, "pitch", "float", 0))
        XCTAssertFalse(chartToggleEnabled(charts, "pitch", "float", 1))
    }

    func testASampleLandsByItsAgeAndValue() {
        XCTAssertTrue(chartPoint(2000, 0.5, 5000, -1.0, 1.0) == (0.6, 0.25))
        XCTAssertTrue(chartPoint(0, -5.0, 5000, -1.0, 1.0) == (1, 1))
    }

    func testTheLegendReadsTheNewestSampleAndNothingBeforeOneArrives() {
        XCTAssertEqual(latestValue([ChartSample(ageMs: 900, value: 0.5), ChartSample(ageMs: 40, value: 0.02), ChartSample(ageMs: 400, value: -1.0)]), "0.02000")
        XCTAssertNil(latestValue([]))
    }

    func testASeriesColourWrapsAnyIndexTheCoreSends() {
        let colours: [Color] = [.red, .green, .blue]
        XCTAssertEqual(seriesColour(1, colours), .green)
        XCTAssertEqual(seriesColour(4, colours), .green)
        XCTAssertEqual(seriesColour(-1, colours), .blue)
        XCTAssertEqual(seriesColour(-3, colours), .red)
    }
}
