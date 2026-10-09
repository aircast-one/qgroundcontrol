import XCTest
@testable import Aircast

final class ScriptingScreenTests: XCTestCase {
    func testThePageListsTheScriptsTheCoreFound() throws {
        let page = try XCTUnwrap(scripting(JSON.parse(#"{"available":true,"enabled":true,"enable":null,"scripts":["hello.lua"],"busy":true,"progress":0.5,"status":"Upload succeeded: /APM/scripts/hello.lua","unsupportedText":"x"}"#)))
        XCTAssertEqual(page.scripts, ["hello.lua"])
        XCTAssertTrue(page.busy)
        XCTAssertEqual(page.progress, 0.5)
        XCTAssertFalse(try XCTUnwrap(scripting(JSON.parse(#"{"available":false}"#))).available)
    }

    func testARefusedTransferOpensTheLuaDialogWithTheCoreTextOrTheFallback() {
        XCTAssertEqual(scriptRefusal("Lua Delete", "Another FTP operation is in progress", "Delete failed"), ScriptRefusal(title: "Lua Delete", text: "Another FTP operation is in progress"))
        XCTAssertEqual(scriptRefusal("Lua Upload", "", "Upload failed"), ScriptRefusal(title: "Lua Upload", text: "Upload failed"))
        XCTAssertNil(scriptRefusal("Lua Download", nil, "Download failed"))
    }

    func testAScriptNameFromTheVehicleStaysInsideTheStagingFolder() {
        XCTAssertEqual(scriptFileName("hello.lua"), "hello.lua")
        XCTAssertEqual(scriptFileName("../../Library/Preferences/x.plist"), ".._.._Library_Preferences_x.plist")
        XCTAssertNil(scriptFileName(".."))
        XCTAssertNil(scriptFileName("."))
        XCTAssertNil(scriptFileName(""))
    }
}
