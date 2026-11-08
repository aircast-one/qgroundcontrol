import XCTest
@testable import Aircast

final class BetaDiagnosticsDialogTests: XCTestCase {
    func testTheOfferShowsWhileTheCoreSaysABetaBuildHasNotAnswered() throws {
        let prompt = try XCTUnwrap(betaDiagnostics(JSON.parse(
            #"{"show":true,"title":"Help us debug this beta","text":"Share logs?","accept":"Share logs","dismiss":"Not now","endpoint":"otel.aircast.one:443"}"#
        )))
        XCTAssertEqual(prompt.title, "Help us debug this beta")
        XCTAssertEqual(prompt.accept, "Share logs")
        XCTAssertEqual(prompt.dismiss, "Not now")
        XCTAssertEqual(prompt.endpoint, "otel.aircast.one:443")
        XCTAssertNil(betaDiagnostics(JSON.parse(#"{"show":false}"#)))
        XCTAssertNil(betaDiagnostics(nil))
    }
}
