import XCTest
@testable import Aircast

final class GeoTagViewTests: XCTestCase {
    private func state(_ body: String) -> GeoTagState {
        geoTagState(JSON.parse(#"{"kind":"object","class":"GeoTagController","# + body + "}"))!
    }

    func testTheButtonReadsAsGeoTagPagesDoes() {
        XCTAssertEqual(geoTagButton(state(#""inProgress":true,"previewMode":true"#)), "Cancel")
        XCTAssertEqual(geoTagButton(state(#""inProgress":false,"previewMode":true"#)), "Preview")
        XCTAssertEqual(geoTagButton(state(#""inProgress":false,"previewMode":false"#)), "Start tagging")
    }

    func testTheSummaryListsOnlyWhatWentWrong() {
        XCTAssertEqual(geoTagSummary(state(#""taggedCount":5"#)), "Successfully tagged 5 images")
        XCTAssertEqual(geoTagSummary(state(#""taggedCount":5,"skippedCount":2,"failedCount":1"#)), "Successfully tagged 5 images (2 skipped, 1 failed)")
        XCTAssertNil(geoTagSummary(state(#""taggedCount":5,"inProgress":true"#)))
        XCTAssertNil(geoTagSummary(state(#""taggedCount":0"#)))
    }

    func testEachImageShowsItsOutcomeAndATaggedOneItsCoordinateAsGeoTagPagesListDoes() {
        let images = state(#""imageModel":[{"fileName":"a.jpg","status":2,"statusString":"Tagged","errorMessage":"","coordinate":{"latitude":-35.1234567,"longitude":149.0}},{"fileName":"b.jpg","status":3,"statusString":"Skipped","errorMessage":"No matching trigger","coordinate":null}]"#).images
        XCTAssertEqual(images.map(\.fileName), ["a.jpg", "b.jpg"])
        XCTAssertEqual(geoTagImageText(images[0]), "Tagged")
        XCTAssertEqual(geoTagImageText(images[1]), "No matching trigger")
        XCTAssertEqual(geoTagCoordinate(images[0]), "-35.123457, 149.000000")
        XCTAssertNil(geoTagCoordinate(images[1]))
    }

    func testTheErrorIsHiddenWhileTaggingRuns() {
        XCTAssertNil(geoTagError(state(#""inProgress":true,"errorMessage":"Tagging cancelled""#)))
        XCTAssertEqual(geoTagError(state(#""inProgress":false,"errorMessage":"Tagging cancelled""#)), "Tagging cancelled")
    }

    func testStepsTickOnceFilledAndOnlyTheCoresImageTypesAreStaged() {
        XCTAssertEqual(geoTagStep(true, 1), "✓")
        XCTAssertEqual(geoTagStep(false, 2), "2")
        XCTAssertTrue(isGeoTagImage("IMG_0001.JPG"))
        XCTAssertTrue(isGeoTagImage("raw.dng"))
        XCTAssertFalse(isGeoTagImage("notes.txt"))
        XCTAssertNil(geoTagState(JSON.parse(#"{"class":"Something"}"#)))
    }
}

final class GeoTagRunTests: XCTestCase {
    func testTheCountComparedIsTheCoresTaggedCountWhichAlreadyLeavesFailuresOut() {
        XCTAssertNil(publishedNote(4, 4))
        XCTAssertEqual(publishedNote(3, 4), "Only 3 of the 4 tagged images could be written to the chosen folder.")
    }

    func testTheOffsetReadsWithEitherDecimalMarkAndIsShownWithAPoint() {
        XCTAssertEqual(parsedOffset("2,5"), 2.5)
        XCTAssertEqual(parsedOffset(" -1.0 "), -1.0)
        XCTAssertNil(parsedOffset("abc"))
        XCTAssertEqual(shownOffset(-3.5), "-3.5")
        XCTAssertNil(parsedOffset("9000"), "DoubleValidator refuses out-of-range input")
        XCTAssertEqual(parsedOffset("-3600"), -3600.0)
        XCTAssertNil(parsedOffset("1.26"), "and a second decimal")
        XCTAssertEqual(parsedOffset(" "), 0.0, "parseFloat(text) || 0")
    }

    func testTheOutputFolderReadsLikeGeoTagPageChosenElseThePhotosTaggedSubfolderElseTheDefault() {
        XCTAssertEqual(geoTagOutputText("Out", "Photos"), "Out")
        XCTAssertEqual(geoTagOutputText(nil, "Photos"), "Photos/TAGGED")
        XCTAssertEqual(geoTagOutputText(nil, nil), "Default: /TAGGED subfolder")
    }
}
