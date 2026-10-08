import XCTest
@testable import Aircast

final class AircastCloudTests: XCTestCase {
    func testFieldsValidateLikeAircastCloudSettingsQml() {
        XCTAssertTrue(cloudApiBaseValid("https://api.aircast.one"))
        XCTAssertTrue(cloudApiBaseValid(" http://10.0.0.2:8080 "))
        XCTAssertFalse(cloudApiBaseValid("api.aircast.one"))
        XCTAssertFalse(cloudApiBaseValid("https:///x"))
        XCTAssertFalse(cloudDeviceValid("  "))
    }

    func testTheAccountLineReadsSignedInTheStatusOrNotSignedIn() throws {
        let signedIn = try XCTUnwrap(accountState(JSON.parse(#"{"kind":"object","signedIn":true,"signingIn":false,"status":"","userCode":"","verificationUrl":""}"#)))
        XCTAssertEqual(accountLine(signedIn), "Signed in")
        XCTAssertEqual(accountLine(AccountState(signedIn: false, signingIn: false, status: "", userCode: "", verificationUrl: "")), "Not signed in")
        XCTAssertEqual(
            accountLine(AccountState(signedIn: false, signingIn: false, status: "Approve code ABC in the browser to sign in.", userCode: "", verificationUrl: "")),
            "Approve code ABC in the browser to sign in."
        )
        XCTAssertNil(accountState(JSON.parse(#"{"kind":"null"}"#)))
    }
}
