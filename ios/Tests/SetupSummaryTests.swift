import XCTest
@testable import Aircast

final class SetupSummaryTests: XCTestCase {
    func testSummaryRowsAreKeyedByTheComponentTheyBelongTo() {
        let read = setupSummaries(JSON.parse(#"{"components":[{"name":"Radio","rows":[{"label":"Roll","value":"Channel 1"},{"label":"Pitch","value":"Setup required"}]},{"name":"Frame","rows":[]}]}"#))
        XCTAssertEqual(read["Radio"], [SummaryLine(label: "Roll", value: "Channel 1"), SummaryLine(label: "Pitch", value: "Setup required")])
        XCTAssertEqual(read["Frame"], [])
        XCTAssertEqual(setupSummaries(nil), [:])
        let joystick = setupSummaries(JSON.parse(#"{"components":[{"name":"Joystick","rows":[{"label":"Battery","value":"15%","warn":true}]}]}"#))
        XCTAssertEqual(joystick["Joystick"], [SummaryLine(label: "Battery", value: "15%", warn: true)])
    }

    func testAWarnedSummaryValueKeepsItsWarningInTheGlanceAsJoystickComponentSummaryPaintsALowBatteryRed() {
        let summary = [SummaryLine(label: "Status", value: "Ready"), SummaryLine(label: "Battery", value: "15% (Discharging)", warn: true)]
        XCTAssertEqual(summaryGlanceParts(summary).map(\.warn), [false, true])
        XCTAssertEqual(summaryGlance(summary), "Ready \u{00b7} 15% (discharging)")
    }

    func testASummaryReadsAsOneLineOfItsValuesAsPenpotsSetupRows() {
        let frame = [SummaryLine(label: "Frame Class", value: "Quad"), SummaryLine(label: "Frame Type", value: "X"), SummaryLine(label: "Firmware Version", value: "Unknown")]
        XCTAssertEqual(summaryGlance(frame), "Quad \u{00b7} X")
        XCTAssertEqual(summaryGlance([SummaryLine(label: "a", value: "RTL"), SummaryLine(label: "b", value: "Land"), SummaryLine(label: "c", value: "RTL")]), "RTL \u{00b7} Land")
        XCTAssertNil(summaryGlance([]))
        XCTAssertEqual(
            summaryGlance([SummaryLine(label: "Battery monitor", value: "Analog Voltage and Current"), SummaryLine(label: "Capacity", value: "5000 mAh")]),
            "Analog voltage and current \u{00b7} 5000 mAh"
        )
        let radio = [SummaryLine(label: "Roll", value: "Channel 1"), SummaryLine(label: "Pitch", value: "Channel 2"), SummaryLine(label: "Throttle", value: "Channel 3")]
        XCTAssertEqual(summaryGlance(radio), "Roll 1 \u{00b7} Pitch 2 \u{00b7} Throttle 3")
        XCTAssertEqual(
            summaryGlance([SummaryLine(label: "Compass", value: "Ready"), SummaryLine(label: "Airspeed", value: "Not Supported(Over APM 4.1)")]),
            "Ready \u{00b7} Not supported (over APM 4.1)"
        )
    }
}
