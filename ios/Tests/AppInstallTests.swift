#if os(macOS)
import XCTest
@testable import Aircast

final class AppInstallTests: XCTestCase {
    func testAnInstallerThatOpenedIsNoFailure() {
        XCTAssertNil(installFailure(fileExists: true, opened: true))
    }

    func testAMissingOrUnopenedInstallerSaysWhyInPlainWords() {
        XCTAssertEqual(installFailure(fileExists: false, opened: false), .Missing)
        XCTAssertEqual(installFailure(fileExists: false, opened: true), .Missing)
        XCTAssertEqual(installFailure(fileExists: true, opened: false), .NotOpened)
        XCTAssertEqual(InstallFailure.NotOpened.text, "macOS could not open the installer.")
        XCTAssertFalse(InstallFailure.Missing.text.isBlank)
    }
}
#endif
