import XCTest
@testable import Aircast

final class TypographyTokensTests: XCTestCase {
    func testEveryTextStyleCarriesItsPenpotSize() {
        XCTAssertEqual([TypeScale.headlineSmall, .titleLarge, .bodyLarge, .bodyMedium, .labelMedium, .labelSmall].map(\.size), [24, 22, 16, 14, 12, 11])
    }
}
