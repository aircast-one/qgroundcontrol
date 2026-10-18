import XCTest
@testable import Aircast

final class UsbRadioIndicatorTests: XCTestCase {
    func testReadsTheCoresRadioStateAndHidesWhileRadioOnUsbIsOff() {
        XCTAssertEqual(usbRadioBadge(JSON.parse(#"{"indicator":{"text":"Radio · live","state":"live"}}"#)), UsbRadioBadge(text: "Radio · live", state: .live))
        XCTAssertNil(usbRadioBadge(JSON.parse(#"{"indicator":null}"#)))
        XCTAssertNil(usbRadioBadge(JSON.parse(#"{"indicator":{"text":"?","state":"exploded"}}"#)))
    }

    func testAStoppedRadioReadsAsAnErrorAndAPausedOneAsAWarning() {
        XCTAssertEqual(usbRadioTone(.stopped), .error)
        XCTAssertEqual(usbRadioTone(.paused), .warning)
        XCTAssertNil(usbRadioTone(.live))
        XCTAssertNil(usbRadioTone(.standby))
    }
}
