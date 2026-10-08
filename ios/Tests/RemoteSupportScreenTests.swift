import XCTest
@testable import Aircast

final class RemoteSupportScreenTests: XCTestCase {
    private func served(_ valid: Bool, _ error: String) -> JSON {
        JSON.parse(#"{"kind":"object","class":"SupportHost","valid":\#(valid),"error":"\#(error)"}"#)
    }

    func testTheHeadReportsTheCoresSentenceNotOneOfItsOwn() throws {
        let refused = try XCTUnwrap(supportHostVerdict(served(false, "The part after the colon has to be a port number between 1 and 65535.")))
        XCTAssertFalse(refused.valid)
        XCTAssertEqual(
            refused.error,
            "The part after the colon has to be a port number between 1 and 65535.",
            "three different refusals each get their own words now; the head used to say 'enter the address your support engineer gave you' to all of them"
        )
    }

    func testAnAcceptedHostCarriesNoComplaint() throws {
        let ok = try XCTUnwrap(supportHostVerdict(served(true, "")))
        XCTAssertTrue(ok.valid)
        XCTAssertEqual(ok.error, "")
    }

    func testAViewOfAnotherShapeIsNotAVerdict() {
        XCTAssertNil(supportHostVerdict(nil))
        XCTAssertNil(
            supportHostVerdict(JSON.parse(#"{"kind":"object","class":"Links"}"#)),
            "answering from whatever happens to be at the path would let an unrelated object enable a button that starts forwarding"
        )
    }

    func testTheQueryNamesTheHostItIsAskingAbout() {
        XCTAssertEqual(supportHostPath("10.0.0.4:14550"), "view.supportHost(10.0.0.4:14550)")
    }

    func testTheCoreAnswersTheCommaNowSoTheHeadDoesNot() {
        XCTAssertNil(
            supportHostCannotBeAsked("10.0.0.4,14550"),
            "view.supportHost sees a typed comma as a second argument - which cannot arise any other way - and refuses it"
        )
        XCTAssertNil(supportHostCannotBeAsked("two words:14550"))
    }

    func testSurroundingSpaceStaysRefusedHereBecauseTheCoreTrimsAndQgcDoesNot() {
        XCTAssertEqual(
            supportHostCannotBeAsked("  leading:14550"),
            "Remove the space before or after the address.",
            "links.rs trims the typed value before judging it, but LinkManager passes the stored value to addHost untrimmed"
        )
        XCTAssertEqual(supportHostCannotBeAsked("host:14550 "), "Remove the space before or after the address.")
        XCTAssertNil(supportHostCannotBeAsked("host:14550"))
        XCTAssertNil(supportHostCannotBeAsked(""), "a blank field is the empty state, not a complaint")
    }
}
