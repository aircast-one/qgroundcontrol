import XCTest
@testable import Aircast

final class RemoteIdIndicatorTests: XCTestCase {
    func testRowsFollowTheIndicatorPage() throws {
        let status = try XCTUnwrap(remoteIdStatus(JSON.parse(#"{"shown":true,"state":"warning","comms":true,"armStatus":true,"gcsGps":false,"basicId":true,"operatorIdShown":false,"operatorId":false,"emergency":false,"emergencyHoldMs":800}"#)))
        XCTAssertEqual(remoteIdRows(status).map(\.0), ["RID COMMS", "ARM STATUS", "GCS GPS", "BASIC ID"])
        XCTAssertEqual(status.holdMs, 800)
        var offline = status
        offline.comms = false
        XCTAssertEqual(remoteIdRows(offline).map(\.0), ["NOT CONNECTED"])
        XCTAssertEqual(remoteIdRows(offline).map(\.1), [false])
        XCTAssertNil(remoteIdStatus(JSON.parse(#"{"shown":false}"#)))
    }

    private func setting(_ name: String, _ value: String) -> Fact {
        Fact(
            path: "settings.remoteIDSettings.\(name)", name: name, description: "", units: "", valueString: value, value: .string(value),
            enumStrings: [], enumIndex: -1, isBool: false, isString: true, readOnly: false
        )
    }

    func testSelfIdListsQgcsFactsInOrderAndGreysTheMessageFieldsWhileBroadcastIsOff() {
        let page = [setting("selfIDFree", "x"), setting("region", "0"), setting("selfIDEmergency", "help"), setting("sendSelfID", "false"), setting("selfIDType", "0")]
        let off = selfIdFacts(page)
        XCTAssertEqual(off.map(\.name), ["sendSelfID", "selfIDType", "selfIDFree", "selfIDEmergency"])
        XCTAssertEqual(off.map(\.acceptsWrite), [true, false, false, true])
        XCTAssertEqual(off.prefix(2).map(\.title), ["Broadcast", "Broadcast message"], "RemoteIDIndicatorPage names the switch and the combo itself")
        let on = page.map { fact -> Fact in
            guard fact.name == "sendSelfID" else { return fact }
            var broadcasting = fact
            broadcasting.valueString = "true"
            broadcasting.value = .bool(true)
            return broadcasting
        }
        XCTAssertEqual(selfIdFacts(on).map(\.acceptsWrite), [true, true, true, true])
    }
}
