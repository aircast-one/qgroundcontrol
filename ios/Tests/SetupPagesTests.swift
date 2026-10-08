import XCTest
@testable import Aircast

private func component(_ name: String, known: String? = nil) -> SetupComponent {
    SetupComponent(index: 0, name: name, known: known, needsAttention: false)
}

final class SetupPagesTests: XCTestCase {
    func testAComponentWithAStableIdentityIsRoutedByItNotByItsDisplayName() {
        XCTAssertEqual(headPage(component("Sensoren", known: "sensors")), SENSORS)
        XCTAssertEqual(headPage(component("Funk", known: "radio")), RADIO)
        XCTAssertEqual(headPage(component("Flugmodi", known: "flightModes")), FLIGHT_MODES_PAGE)
    }

    func testAComponentQgcHasNoIdentityForFallsBackToItsNameWhichIsAllThereIs() {
        XCTAssertEqual(headPage(component(MOTORS)), MOTORS)
        XCTAssertEqual(headPage(component(REMOTE_SUPPORT)), REMOTE_SUPPORT)
    }

    func testAnIdentityTheHeadHasNoPageForKeepsTheNameRatherThanResolvingToNothing() {
        XCTAssertEqual(headPage(component("Power", known: "power")), "Power")
        XCTAssertEqual(headPage(component("Safety", known: "safety")), "Safety")
    }
}
