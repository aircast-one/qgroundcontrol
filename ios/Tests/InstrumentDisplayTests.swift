import UIKit
import XCTest
@testable import Aircast

final class InstrumentDisplayTests: XCTestCase {
    private let ranged = ValueDisplay(rangeType: .Color, values: [10.0, 20.0], colours: [1, 2, 3])

    func testAValueTakesTheColourOfTheRangeItFallsIn() {
        XCTAssertEqual(displayColour(ranged, 5.0), 1)
        XCTAssertEqual(displayColour(ranged, 20.0), 2)
        XCTAssertEqual(displayColour(ranged, 25.0), 3)
        XCTAssertEqual(displayColour(ranged, nil), 1, "an unknown value is the first range, as QGC treats NaN")
        XCTAssertNil(displayColour(ValueDisplay(), 5.0))
        XCTAssertNil(displayColour(withChanges(ranged) { $0.colours = [NO_COLOUR, 2, 3] }, -1.0), "an unchecked colour slot falls back to the palette colour")
    }

    func testForwardFlightVehiclesKeepAirspeedAmongTheirDefaultValuesLikeQgcCorePlugin() {
        XCTAssertEqual(defaultInstruments("fixedWing"), DEFAULT_INSTRUMENTS + ["airSpeed"])
        XCTAssertEqual(defaultInstruments("multiRotor"), DEFAULT_INSTRUMENTS)
    }

    func testRemovingAThresholdDropsTheBandAboveItAsInstrumentValueDataRemoveRangeValueDoes() {
        let coloured = ValueDisplay(rangeType: .Color, values: [0.0, 50.0], colours: [1, 2, 3])
        XCTAssertEqual(withoutRow(coloured, 0).colours, [1, 3])
    }

    func testSwitchingTheRangeTypeResetsTheRowsLikeResetRangeInfo() {
        let icons = withRangeType(ValueDisplay(), .Icon, "airplane.svg")
        XCTAssertEqual(icons.values, [0.0, 100.0])
        XCTAssertEqual(icons.icons, Array(repeating: "airplane.svg", count: 3))
        XCTAssertTrue(icons.colours.isEmpty)
        XCTAssertEqual(withRow(icons, "x.svg").values.count, 3)
        XCTAssertEqual(withRow(icons, "x.svg").icons.last, "x.svg")
        XCTAssertEqual(withoutRow(icons, 0).values, [100.0])
        XCTAssertEqual(withRangeType(icons, .None, "airplane.svg"), ValueDisplay())
    }

    func testOpacityAndIconRangesPickByValueAndAFixedIconReplacesTheLabel() {
        let faded = withChanges(withRangeType(ValueDisplay(), .Opacity, "a.svg")) { $0.opacities = [0.2, 0.5, 1.0] }
        XCTAssertEqual(displayOpacity(faded, -1.0), 0.2)
        XCTAssertEqual(displayOpacity(faded, 150.0), 1)
        XCTAssertEqual(displayOpacity(ValueDisplay(), 5.0), 1)
        let swapped = withChanges(withRangeType(ValueDisplay(), .Icon, "a.svg")) { $0.icons = ["low.svg", "mid.svg", "high.svg"] }
        XCTAssertEqual(displayIcon(swapped, 50.0), "mid.svg")
        XCTAssertEqual(displayIcon(ValueDisplay(showIcon: true, icon: "plane.svg"), 5.0), "plane.svg")
        XCTAssertNil(displayIcon(ValueDisplay(), 5.0))
    }

    func testADisplaySurvivesARoundTripThroughStorage() {
        let display = ValueDisplay(text: "Alt", showIcon: true, icon: "plane.svg", rangeType: .Opacity, values: [1.0], opacities: [0.5, 1.0])
        XCTAssertEqual(displayFrom(displayJson(display)), display)
    }

    func testEveryBundledIconIsListedAndLoads() {
        XCTAssertTrue(iconNames().contains("airplane.svg"))
        XCTAssertNotNil(UIImage(named: "InstrumentValueIcons/airplane"))
    }
}
