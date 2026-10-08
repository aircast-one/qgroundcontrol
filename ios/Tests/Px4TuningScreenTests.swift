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
}
