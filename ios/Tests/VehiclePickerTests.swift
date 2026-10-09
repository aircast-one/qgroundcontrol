import XCTest
@testable import Aircast

final class VehiclePickerTests: XCTestCase {
    override func tearDown() {
        VehicleBridge.forget()
        super.tearDown()
    }

    func testAnAcceptedAskIsRememberedAndARefusalKeepsTheReasonTheCoreGave() {
        XCTAssertFalse(VehicleBridge.answered(3, JSON.parse(#"{"ok":false,"reason":"Vehicle 3 is not answering"}"#)))
        XCTAssertEqual(VehicleBridge.lastRefusal, "Vehicle 3 is not answering")
        XCTAssertNil(VehicleBridge.lastAsked)
        XCTAssertTrue(VehicleBridge.answered(4, JSON.parse(#"{"ok":true,"reason":""}"#)))
        XCTAssertNil(VehicleBridge.lastRefusal)
        XCTAssertEqual(VehicleBridge.lastAsked, 4)
        XCTAssertFalse(VehicleBridge.answered(5, nil))
        XCTAssertEqual(VehicleBridge.lastRefusal, "bridge threw: no answer")
        XCTAssertEqual(VehicleBridge.lastAsked, 4, "a refused ask keeps the vehicle last switched to")
        VehicleBridge.forget()
        XCTAssertNil(VehicleBridge.lastAsked)
    }

    func testAsksFromManyThreadsWhileTheMainThreadForgetsNeverTearTheRememberedState() {
        let refusals: Set<String?> = [nil, "busy"]
        _ = VehicleBridge.answered(0, JSON.parse(#"{"ok":false,"reason":"busy"}"#))
        DispatchQueue.concurrentPerform(iterations: 2000) { at in
            switch at % 4 {
            case 0: _ = VehicleBridge.answered(at, JSON.parse(#"{"ok":true}"#))
            case 1: _ = VehicleBridge.answered(at, JSON.parse(#"{"ok":false,"reason":"busy"}"#))
            case 2: VehicleBridge.forget()
            default: XCTAssertTrue(refusals.contains(VehicleBridge.lastRefusal))
            }
        }
        XCTAssertTrue(refusals.contains(VehicleBridge.lastRefusal))
    }
}
