import XCTest
@testable import Aircast

final class ThermalViewTests: XCTestCase {
    private func camera(available: Bool = true, mode: String = #""blend""#, opacity: String = "60.0") -> JSON {
        JSON.parse(#"{"kind":"object","class":"Camera","present":true,"thermalAvailable":\#(available),"thermalMode":\#(mode),"thermalOpacity":\#(opacity)}"#)
    }

    func testACameraWithNoThermalStreamOffersNoThermalControls() {
        XCTAssertNil(thermalReading(camera(available: false, mode: "null", opacity: "null")), "a plain camera has no mode rather than a mode of off")
        XCTAssertNil(thermalReading(nil))
    }

    func testTheServedTokenIsTheModeNotAnIndexIntoAList() {
        XCTAssertEqual(thermalReading(camera())!.mode, "blend")
        XCTAssertEqual(thermalReading(camera(mode: #""picInPic""#))!.mode, "picInPic")
    }

    func testEveryServedTokenHasAWordAndAnUnknownOneIsShownRatherThanHidden() {
        XCTAssertEqual(thermalModeLabel("off"), "Off")
        XCTAssertEqual(thermalModeLabel("blend"), "Blend")
        XCTAssertEqual(thermalModeLabel("full"), "Full")
        XCTAssertEqual(thermalModeLabel("picInPic"), "Picture in picture")
        XCTAssertEqual(thermalModeLabel("somethingNew"), "somethingNew", "a token the core adds later is drawn as itself")
    }

    func testOpacityIsOfferedOnlyWhenTheCoreSendsOne() {
        XCTAssertTrue(thermalOpacityIsOffered(thermalReading(camera())), "a null means do not draw the slider, not draw it at zero")
        XCTAssertFalse(thermalOpacityIsOffered(thermalReading(camera(mode: #""full""#, opacity: "null"))))
        XCTAssertFalse(thermalOpacityIsOffered(nil))
    }

    func testANullOpacityIsAbsentNotZero() {
        let full = thermalReading(camera(mode: #""full""#, opacity: "null"))!
        XCTAssertEqual(full.mode, "full")
        XCTAssertNil(full.opacity, "a slider at zero claims the blend is fully transparent, which is a different statement")
    }

    func testTheWriteOrderMatchesTheTokensTheCoreServes() {
        XCTAssertEqual(THERMAL_MODES, ["off", "blend", "full", "picInPic"])
        XCTAssertEqual(THERMAL_MODES.firstIndex(of: "blend"), 1, "the property is a Q_ENUM written by index")
    }
}
