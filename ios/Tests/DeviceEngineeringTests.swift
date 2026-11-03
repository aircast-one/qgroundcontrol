import XCTest
@testable import Aircast

final class DeviceEngineeringTests: XCTestCase {
    private func field(_ name: String, _ value: String = "", secret: Bool = false) -> String {
        #"{"name":"\#(name)","label":"\#(name)","value":"\#(value)","placeholder":"","secret":\#(secret)}"#
    }

    private var served: String {
        """
        {
          "backend": {"status": {"level": 3, "text": "mediamtx"}, "text": "Video is carried by mediamtx.", "switch": "engine", "label": "Switch to the engine", "confirm": "Restart?", "success": "Switching."},
          "publish": {
            "status": {"level": 3, "text": "awaiting cloud connect"},
            "awaitingCloud": true,
            "mode": "aircast",
            "modes": [
              {"value": "aircast", "title": "Aircast cloud", "desc": "Managed.", "fields": [], "clears": ["token"], "submit": "Test sign-in", "unsaved": "Unsaved change — press Test sign-in to apply."},
              {"value": "oauth", "title": "Custom OAuth server", "desc": "OAuth.", "fields": ["issuer", "clientId", "clientSecret", "tokenUrl", "scope"], "clears": ["token"], "submit": "Save", "unsaved": "Unsaved."},
              {"value": "token", "title": "Access token (WHIP)", "desc": "Token.", "fields": ["token"], "clears": [], "submit": "Save", "unsaved": "Unsaved."},
              {"value": "open", "title": "None — open server", "desc": "Open.", "fields": [], "clears": ["issuer", "clientId", "clientSecret", "tokenUrl", "scope", "token"], "submit": "Save", "unsaved": "Unsaved.", "confirm": "Clear all stored sign-in settings, including the cloud issuer?"}
            ],
            "fields": [\(field("issuer", "https://api.dev.aircast.one")), \(field("clientId", "drone")), \(field("clientSecret", "s3cr3t", secret: true)), \(field("tokenUrl")), \(field("scope")), \(field("token", "old-token", secret: true))],
            "environment": "https://api.dev.aircast.one",
            "verdict": {"level": 3, "text": "Not tested yet — press Test sign-in."}
          },
          "diagnostics": {"status": {"level": 3, "text": "off"}, "hint": "Stream logs.", "endpoint": "otel.aircast.one:443", "token": "", "tokenLabel": "Bearer token", "streaming": false, "canStream": false, "state": "Save the endpoint first.", "confirm": "Stream?"}
        }
        """
    }

    private var view: EngineeringView? { parseEngineering(JSON.parse(served)) }

    private func servedPublish() throws -> PublishSignIn { try XCTUnwrap(view).publish }

    func testReadsTheEngineeringPageTheDeviceServes() throws {
        let publish = try servedPublish()
        XCTAssertEqual(view?.backend.switchTo, "engine")
        XCTAssertEqual(publish.mode, "aircast")
        XCTAssertTrue(publish.awaitingCloud)
        XCTAssertEqual(publish.fields.first { $0.name == "clientSecret" }?.secret, true)
        XCTAssertFalse(view?.diagnostics.canStream ?? true)
        XCTAssertNil(parseEngineering(JSON.parse(#"{"publish":{}}"#)))
    }

    func testEachSignInMethodShowsOnlyItsOwnFields() throws {
        let publish = try servedPublish()
        XCTAssertEqual(signInFields(publish, "aircast"), [])
        XCTAssertEqual(signInFields(publish, "oauth").map(\.name), ["issuer", "clientId", "clientSecret", "tokenUrl", "scope"])
        XCTAssertEqual(signInFields(publish, "token").map(\.name), ["token"])
        XCTAssertEqual(modeNamed(publish, "aircast")?.submit, "Test sign-in")
        XCTAssertNil(modeNamed(publish, "nowhere"))
    }

    func testSavingSendsEveryCredentialAndEmptiesExactlyWhatTheMethodClears() throws {
        let publish = try servedPublish()
        XCTAssertEqual(
            signInBody(publish, "aircast", [:]),
            ["issuer": "https://api.dev.aircast.one", "clientId": "drone", "clientSecret": "s3cr3t", "tokenUrl": "", "scope": "", "token": ""]
        )
        XCTAssertEqual(
            signInBody(publish, "token", ["token": "whip-token-abc"]),
            ["issuer": "https://api.dev.aircast.one", "clientId": "drone", "clientSecret": "s3cr3t", "tokenUrl": "", "scope": "", "token": "whip-token-abc"]
        )
        XCTAssertEqual(
            signInBody(publish, "open", ["issuer": "typed"]),
            ["issuer": "", "clientId": "", "clientSecret": "", "tokenUrl": "", "scope": "", "token": ""]
        )
        let oauth = signInBody(publish, "oauth", ["clientSecret": "new"])
        XCTAssertEqual(oauth["clientSecret"], "new")
        XCTAssertEqual(oauth["token"], "")
    }

    func testTheEditorStartsOverOnlyWhenWhatIsSavedChanges() throws {
        let publish = try servedPublish()
        XCTAssertEqual(signInKey(publish), signInKey(try XCTUnwrap(parseEngineering(JSON.parse(served))).publish))
        XCTAssertNotEqual(signInKey(publish), signInKey(withChanges(publish) { $0.mode = "token" }))
        XCTAssertNotEqual(
            signInKey(publish),
            signInKey(withChanges(publish) { edited in
                edited.fields = edited.fields.map { field in field.name == "token" ? withChanges(field) { $0.value = "new" } : field }
            })
        )
    }
}
