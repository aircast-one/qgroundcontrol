import XCTest
@testable import Aircast

final class Px4TuningScreenTests: XCTestCase {
    func testTabsAxesAndSlidersReadFromTheTuningView() throws {
        let tabs = tuningTabs(JSON.parse(
            #"{"available":true,"tabs":[{"name":"Rate Controller","title":"Rate","unit":"deg/s","extras":[],"axes":[{"name":"Roll","params":["#
                + #"{"title":"Overall Multiplier (MC_ROLLRATE_K)","description":"d","param":"MC_ROLLRATE_K","min":0.3,"max":3,"step":0.05,"#
                + #""fact":{"class":"Control","path":"vehicle.parameterManager.getParameter(-1,MC_ROLLRATE_K)","name":"MC_ROLLRATE_K","value":1,"valueString":"1.00"}}]}]}]}"#
        ))
        XCTAssertEqual(tabs.count, 1)
        XCTAssertEqual(tabs.first?.name, "Rate controller")
        let param = try XCTUnwrap(tabs.first?.axes.first?.params.first)
        XCTAssertEqual(param.fact.name, "MC_ROLLRATE_K")
        XCTAssertEqual(param.title, "Overall multiplier (MC_ROLLRATE_K)")
        XCTAssertEqual(factNumber(param.fact), 1)
        XCTAssertEqual(sliderSteps(param.min, param.max, param.step), 53)
    }

    func testASliderWhoseLimitsTheCoreLeftOutStillEqualsItselfSoTheChartIsNotRestartedEveryTick() {
        let view = JSON.parse(
            #"{"available":true,"tabs":[{"name":"Rate","axes":[{"name":"Roll","params":[{"title":"P","fact":{"class":"Control","path":"p","name":"P","value":1}}]}]}]}"#
        )
        let param = tuningTabs(view).first?.axes.first?.params.first
        XCTAssertEqual(param?.min.isNaN, true)
        XCTAssertEqual(tuningTabs(view), tuningTabs(view))
    }

    func testAStepTooFineToCountSaturatesInsteadOfTrapping() {
        XCTAssertEqual(sliderSteps(0, 3e38, 1e-30), Int.max - 1)
        XCTAssertEqual(sliderSteps(0, 1, 5), 0)
        XCTAssertEqual(sliderSteps(1, 0, 0.1), 0)
        XCTAssertEqual(sliderSteps(.nan, 1, 0.1), 0)
    }
}
