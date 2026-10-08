import XCTest
@testable import Aircast

final class LoggingCategoriesTests: XCTestCase {
    private let served = JSON.parse(
        #"""
        {"active":["groundstation::hub"],"categories":[
            {"name":"groundstation","shortName":"groundstation","depth":0,"enabled":false},
            {"name":"groundstation::hub","shortName":"hub","depth":1,"enabled":true},
            {"name":"ureq","shortName":"ureq","depth":0,"enabled":false}]}
        """#
    )

    func testTheTreeReadsWithItsDepthsAndTheActiveList() throws {
        let read = try XCTUnwrap(logCategories(served))
        XCTAssertEqual(read.active, ["groundstation::hub"])
        XCTAssertEqual(read.categories.map(\.depth), [0, 1, 0])
        XCTAssertEqual(read.categories.map(\.enabled), [false, true, false])
    }

    func testSearchingMatchesAnywhereInTheFullNameIgnoringCase() throws {
        let read = try XCTUnwrap(logCategories(served))
        XCTAssertEqual(filteredCategories(read.categories, "HUB").map(\.name), ["groundstation::hub"])
        XCTAssertEqual(filteredCategories(read.categories, "  ").map(\.name), [], "QSortFilterProxyModel::setFilterFixedString matches the text as typed")
    }

    func testTheTreeStartsCollapsedListsChildrenUnderTheirParentAndOpensOneLevelPerExpandedParentLikeTreeView() throws {
        let read = try XCTUnwrap(logCategories(JSON.parse(
            #"""
            {"active":[],"categories":[
                {"name":"a","shortName":"a","depth":0,"enabled":false},
                {"name":"a1","shortName":"a1","depth":0,"enabled":false},
                {"name":"a::b","shortName":"b","depth":1,"enabled":false},
                {"name":"a::b::c","shortName":"c","depth":2,"enabled":false},
                {"name":"a1::x","shortName":"x","depth":1,"enabled":false}]}
            """#
        )))
        XCTAssertEqual(parentNames(read.categories), ["a", "a::b", "a1"])
        XCTAssertEqual(read.categories.last.flatMap { parentOf(read.categories, $0) }?.name, "a1")
        XCTAssertEqual(shownInTree(read.categories, []).map(\.name), ["a", "a1"])
        XCTAssertEqual(shownInTree(read.categories, ["a"]).map(\.name), ["a", "a::b", "a1"])
        XCTAssertEqual(shownInTree(read.categories, ["a::b"]).map(\.name), ["a", "a1"], "a child stays hidden while its parent is collapsed")
        XCTAssertEqual(shownInTree(read.categories, ["a", "a::b"]).map(\.name), ["a", "a::b", "a::b::c", "a1"])
    }
}
