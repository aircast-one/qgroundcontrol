import XCTest
@testable import Aircast

final class RadioLinkTests: XCTestCase {
    private func state(_ extra: String) -> FlyState? {
        flyState(JSON.parse(
            #"{"kind":"object","class":"FlyState","connected":true,"armed":false,"contactLost":false,"state":"ready","stateText":"Ready","staleNotice":"","mode":"Stabilize","rcSupported":true,"rcSignalText":"80%","rcSignal":80,"# + extra + "}"
        ))
    }

    func testARadioThatHasNeverSpokenIsNotALinkAtMinusZero() {
        XCTAssertNil(state(#""rcOverride":false,"telemetry":null"#)?.telemetry)
        XCTAssertNil(telemetryCell(state(#""rcOverride":false,"telemetry":null"#)))
        XCTAssertEqual(telemetryDetail(nil), [])
    }

    func testTheStripShowsThisStationsOwnSignal() {
        let reading = state(#""rcOverride":false,"telemetry":{"localRssiDbm":-71,"remoteRssiDbm":-68,"localNoise":42,"remoteNoise":39,"receiveErrors":7,"errorsFixed":2,"txBuffer":95}"#)
        XCTAssertEqual(telemetryCell(reading), "-71 dBm")
        XCTAssertEqual(
            telemetryDetail(reading?.telemetry).map(\.label),
            ["Local RSSI:", "Remote RSSI:", "RX Errors:", "Errors Fixed:", "TX Buffer:", "Local Noise:", "Remote Noise:"]
        )
    }

    func testARadioReportingOnlyItsOwnEndStillGivesAReadingAndAShorterDetail() {
        let reading = state(#""rcOverride":false,"telemetry":{"localRssiDbm":-71,"remoteRssiDbm":null,"localNoise":null,"remoteNoise":null,"receiveErrors":null}"#)
        XCTAssertEqual(telemetryCell(reading), "-71 dBm")
        XCTAssertEqual(telemetryDetail(reading?.telemetry).map(\.label), ["Local RSSI:"])
    }

    func testTheOverrideCellAppearsOnlyWhileTheTransmitterIsActuallyOverridden() {
        XCTAssertEqual(overrideCell(state(#""rcOverride":true,"telemetry":null"#))?.text, "RC override")
        XCTAssertNil(overrideCell(state(#""rcOverride":false,"telemetry":null"#)))
        XCTAssertNil(
            overrideCell(state(#""rcOverride":null,"telemetry":null"#)),
            "null is no vehicle, and claiming manual control is not overridden would be a safety claim from nothing"
        )
    }
}
