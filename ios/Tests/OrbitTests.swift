import XCTest
@testable import Aircast

final class OrbitTests: XCTestCase {
    private func view(orbiting: String = "true", radiusText: String = #""120 m""#, clockwise: String = "true", reason: String = "\"\"") -> JSON {
        JSON.parse(#"{"kind":"object","class":"Orbit","available":true,"orbiting":\#(orbiting),"radiusText":\#(radiusText),"clockwise":\#(clockwise),"reason":\#(reason)}"#)
    }

    func testAnotherViewIsNotAnOrbitReading() {
        XCTAssertNil(orbitReading(nil))
        XCTAssertNil(orbitReading(JSON.parse(#"{"kind":"object","class":"Orbit2"}"#)))
    }

    func testAnOrbitStatesItsRadiusAndWhichWayRoundItGoes() {
        XCTAssertEqual(orbitLabel(orbitReading(view())), "Orbiting 120 m clockwise")
        XCTAssertEqual(orbitLabel(orbitReading(view(clockwise: "false"))), "Orbiting 120 m anticlockwise")
    }

    func testAVehicleNotOrbitingShowsNothingBecauseTheChipIsTheState() {
        XCTAssertNil(orbitLabel(orbitReading(view(orbiting: "false", radiusText: "null", clockwise: "null"))))
    }

    func testNoContactIsNotAStoppedOrbitAndNeitherDrawsTheChip() {
        let quiet = orbitReading(view(orbiting: "null", radiusText: "null", clockwise: "null", reason: #""No contact, so whether the vehicle is still orbiting is unknown.""#))!
        XCTAssertNil(quiet.orbiting)
        XCTAssertNil(orbitLabel(quiet))
        XCTAssertEqual(quiet.reason, "No contact, so whether the vehicle is still orbiting is unknown.")
    }

    func testAnOrbitWhoseRadiusTheCoreWillNotStateIsStillAnOrbit() {
        XCTAssertEqual(orbitLabel(orbitReading(view(radiusText: "null"))), "Orbiting clockwise")
    }
}
