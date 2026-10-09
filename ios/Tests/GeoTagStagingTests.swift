import XCTest
@testable import Aircast

final class GeoTagStagingTests: XCTestCase {
    private var root: URL!

    override func setUpWithError() throws {
        root = FileManager.default.temporaryDirectory.appending(path: "geotag-staging-\(UUID().uuidString)", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: root, withIntermediateDirectories: true)
    }

    override func tearDownWithError() throws {
        try? FileManager.default.removeItem(at: root)
    }

    private func folder(_ path: String) throws -> URL {
        let made = root.appending(path: path, directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: made, withIntermediateDirectories: true)
        return made
    }

    @discardableResult
    private func file(_ dir: URL, _ name: String, _ text: String = "x", modified: Date? = nil) throws -> URL {
        let made = dir.appending(path: name)
        try Data(text.utf8).write(to: made)
        if let modified { try FileManager.default.setAttributes([.modificationDate: modified], ofItemAtPath: made.path) }
        return made
    }

    private func names(_ dir: URL) throws -> [String] {
        try FileManager.default.contentsOfDirectory(atPath: dir.path).sorted()
    }

    func testStagingCopiesOnlyTheTopLevelImagesOfTheFolder() throws {
        let tree = try folder("survey")
        try ["a.jpg", "b.DNG", "c.tif", "notes.txt"].forEach { try file(tree, $0) }
        try file(try folder("survey/sub"), "d.jpg")
        let (path, count) = stageImages(tree)
        XCTAssertEqual(count, 3)
        XCTAssertEqual(try names(URL(fileURLWithPath: path)), ["a.jpg", "b.DNG", "c.tif"])
    }

    func testRestagingReplacesThePreviousImages() throws {
        let first = try folder("first")
        try file(first, "old.jpg")
        _ = stageImages(first)
        let second = try folder("second")
        try file(second, "new.jpg")
        let (path, count) = stageImages(second)
        XCTAssertEqual(count, 1)
        XCTAssertEqual(try names(URL(fileURLWithPath: path)), ["new.jpg"])
    }

    func testALogIsStagedUnderItsNameOrTheFallback() throws {
        let log = try file(try folder("logs"), "picked.ulg", "flight")
        let named = try XCTUnwrap(stageLog(log, "flight.ulg"))
        XCTAssertEqual(URL(fileURLWithPath: named).lastPathComponent, "flight.ulg")
        XCTAssertEqual(try String(contentsOfFile: named, encoding: .utf8), "flight")
        XCTAssertEqual(stageLog(log, "").map { URL(fileURLWithPath: $0).lastPathComponent }, "flight.log")
        XCTAssertNil(stageLog(root.appending(path: "missing.ulg"), "missing.ulg"))
    }

    func testTheTaggedFolderAndExistingImagesAreSeen() throws {
        let tree = try folder("tree")
        XCTAssertFalse(hasTaggedFolder(tree))
        XCTAssertFalse(holdsImages(tree))
        try file(tree, DEFAULT_GEOTAG_OUTPUT)
        try file(tree, "readme.txt")
        XCTAssertFalse(hasTaggedFolder(tree), "a file named like the folder is not the folder")
        XCTAssertFalse(holdsImages(tree))
        let other = try folder("other/\(DEFAULT_GEOTAG_OUTPUT)").deletingLastPathComponent()
        try file(other, "a.JPEG")
        XCTAssertTrue(hasTaggedFolder(other))
        XCTAssertTrue(holdsImages(other))
    }

    func testPublishingCreatesTheTaggedSubfolderAndWritesOnlyImages() throws {
        let staged = try folder("staged")
        try file(staged, "x.jpg", "tagged")
        try file(staged, "y.txt")
        let tree = try folder("out")
        XCTAssertEqual(publishTagged(staged, tree, DEFAULT_GEOTAG_OUTPUT), 1)
        XCTAssertEqual(try names(tree), [DEFAULT_GEOTAG_OUTPUT])
        XCTAssertEqual(try names(tree.appending(path: DEFAULT_GEOTAG_OUTPUT)), ["x.jpg"])
    }

    func testPublishingReusesTheSubfolderAndReplacesItsImages() throws {
        let staged = try folder("staged")
        try file(staged, "x.jpg", "tagged")
        let tree = try folder("out")
        let tagged = try folder("out/\(DEFAULT_GEOTAG_OUTPUT)")
        try file(tagged, "x.jpg", "untagged")
        try file(tagged, "kept.jpg", "kept")
        XCTAssertEqual(publishTagged(staged, tree, DEFAULT_GEOTAG_OUTPUT), 1)
        XCTAssertEqual(try String(contentsOf: tagged.appending(path: "x.jpg"), encoding: .utf8), "tagged")
        XCTAssertEqual(try names(tagged), ["kept.jpg", "x.jpg"])
    }

    func testPublishingWithoutASubfolderWritesIntoTheChosenFolder() throws {
        let staged = try folder("staged")
        try file(staged, "x.tif")
        let tree = try folder("chosen")
        XCTAssertEqual(publishTagged(staged, tree, nil), 1)
        XCTAssertEqual(try names(tree), ["x.tif"])
    }

    func testAFailedWriteIsNotCountedAndLeavesWhatWasAlreadyThere() throws {
        let staged = try folder("staged")
        try file(staged, "blocked.jpg")
        try file(staged, "fine.jpg")
        let tree = try folder("out")
        let blocker = try folder("out/blocked.jpg")
        try file(blocker, "inside.txt")
        XCTAssertEqual(publishTagged(staged, tree, nil), 1)
        XCTAssertEqual(try names(blocker), ["inside.txt"])
        XCTAssertEqual(try names(tree), ["blocked.jpg", "fine.jpg"])
    }

    func testDownloadedLogsAreFlightLogsNewestFirst() throws {
        let logs = try folder("saved")
        try file(logs, "old.ulg", modified: Date(timeIntervalSince1970: 1_000))
        try file(logs, "new.BIN", modified: Date(timeIntervalSince1970: 2_000))
        try file(logs, "notes.txt", modified: Date(timeIntervalSince1970: 3_000))
        _ = try folder("saved/dir.ulg")
        XCTAssertEqual(downloadedLogs(logs.path).map(\.lastPathComponent), ["new.BIN", "old.ulg"])
        XCTAssertEqual(downloadedLogs(root.appending(path: "missing").path), [])
    }
}
