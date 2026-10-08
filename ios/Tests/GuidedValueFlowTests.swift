import XCTest
@testable import Aircast

final class GuidedValueFlowTests: XCTestCase {
    private let kind = GuidedValueKind(offerId: "takeoff", title: "Takeoff", prompt: nil, commitLabel: "Take off", missingRange: "no range", quickPicks: false, read: { _ in nil }, commit: { _ in })

    func testAValueOpensOnlyWithAUsableRangeAndAStartingValue() {
        let usable = GuidedReading(label: "Height", unit: "m", range: 1.0...120.0, initial: 10.0, sentence: "", sendable: true)
        XCTAssertNotNil(openedGuidedValue(kind, usable))
        XCTAssertEqual(openedGuidedValue(kind, usable)!.target, 10.0, accuracy: 1e-9)
        var rangeless = usable
        rangeless.range = nil
        XCTAssertNil(openedGuidedValue(kind, rangeless))
        var startless = usable
        startless.initial = nil
        XCTAssertNil(openedGuidedValue(kind, startless))
        XCTAssertNil(openedGuidedValue(kind, nil))
    }

    func testEachGuidedReadingMapsOntoTheSharedShape() {
        let takeoff = takeoffReading(GuidedTakeoff(label: "Height", unit: "ft", initial: 10.0, minimum: 10.0, maximum: 400.0, sentence: "climb", targetMeters: 3.0))!
        XCTAssertEqual(takeoff.range, 10.0...400.0)
        XCTAssertEqual(takeoff.initial, 10.0)
        XCTAssertNil(takeoffReading(GuidedTakeoff(label: "Height", unit: "ft", initial: 10.0, minimum: 400.0, maximum: 10.0, sentence: "", targetMeters: 3.0))!.range)

        let altitude = altitudeReading(GuidedAltitude(available: true, label: "Alt", unit: "m", current: 25.0, minimum: 1.0, maximum: 120.0, sentence: "", deltaMeters: 0.0, sends: false))!
        XCTAssertEqual(altitude.initial, 25.0)
        XCTAssertFalse(altitude.sendable)
    }
}
