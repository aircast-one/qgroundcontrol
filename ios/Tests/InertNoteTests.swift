import XCTest
@testable import Aircast

final class InertNoteTests: XCTestCase {
    private func fact(_ name: String, enabled: Bool = true, readOnly: Bool = false, isBool: Bool = true) -> Fact {
        Fact(
            path: "settings.x.\(name)", name: name, description: "", units: "", valueString: "", value: .null,
            enumStrings: [], enumIndex: -1, isBool: isBool, isString: false, readOnly: readOnly, enabled: enabled
        )
    }

    private func changed(_ fact: Fact, _ change: (inout Fact) -> Void) -> Fact {
        var copy = fact
        change(&copy)
        return copy
    }

    func testTheCoresOwnDisabledReasonIsTheNote() {
        XCTAssertEqual(
            inertNote(changed(fact("enforceChecklist", enabled: false)) { $0.disabledReason = "Has no effect while the preflight checklist is off." }),
            "Has no effect while the preflight checklist is off."
        )
        XCTAssertEqual(inertNote(fact("enforceChecklist", enabled: false)), "Has no effect yet")
    }

    func testAReadOnlyFactSaysSo() {
        XCTAssertEqual(inertNote(fact("useChecklist", readOnly: true)), "Read-only")
    }

    func testEverySettingIsBuiltHereSoNoneIsCalledInertWhileEnabled() {
        XCTAssertEqual(inertNote(fact("displayPresetsTabFirst")), "Read-only")
        XCTAssertTrue(showsAsField(fact("displayPresetsTabFirst", isBool: false)))
    }

    func testAnUnknownControlKindSaysEditOnDesktop() {
        XCTAssertTrue(editOnDesktop(changed(fact("useChecklist")) { $0.controlKind = "slider" }))
    }

    func testASubtitleCarriesOnlyTheUnits() {
        XCTAssertEqual(factSubtitle(changed(fact("someLiveNumber", isBool: false)) { $0.units = "MB" }), "MB")
        XCTAssertEqual(factSubtitle(fact("someLiveToggle")), "")
    }
}
