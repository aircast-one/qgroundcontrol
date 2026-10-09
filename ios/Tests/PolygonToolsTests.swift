import XCTest
@testable import Aircast

final class PolygonToolsTests: XCTestCase {
    private let small = [TrackPoint(47.401, 8.499), TrackPoint(47.401, 8.501), TrackPoint(47.399, 8.501), TrackPoint(47.399, 8.499)]

    func testRectangleFillsThreeQuartersOfTheView() {
        let corners = defaultRectangle(small)
        XCTAssertEqual(4, corners.count)
        XCTAssertEqual(metresBetween(small[0], small[1]) * 0.75, metresBetween(corners[0], corners[1]), accuracy: 1.0)
        XCTAssertTrue(corners[0].latitude > corners[3].latitude && corners[0].longitude < corners[1].longitude)
    }

    func testAZoomedOutViewCapsTheShapeAtThreeKilometres() {
        let wide = [TrackPoint(48.0, 8.0), TrackPoint(48.0, 9.0), TrackPoint(47.0, 9.0), TrackPoint(47.0, 8.0)]
        XCTAssertEqual(3000.0, metresBetween(defaultRectangle(wide)[0], defaultRectangle(wide)[1]), accuracy: 5.0)
    }

    func testCircleHasSixteenCornersAtTheSmallerHalfExtent() {
        let ring = defaultCircle(small)
        XCTAssertEqual(16, ring.count)
        let centre = TrackPoint(47.4, 8.5)
        let half = min(metresBetween(small[0], small[1]), metresBetween(small[0], small[3])) * 0.75 / 2
        ring.forEach { XCTAssertEqual(half, metresBetween(centre, $0), accuracy: 1.0) }
    }

    func testNoViewNoShape() {
        XCTAssertTrue(defaultRectangle([]).isEmpty)
        XCTAssertTrue(defaultCircle(Array(small.prefix(3))).isEmpty)
    }

    func testShapePathNamesTheFenceOrTheSurveyArea() {
        XCTAssertEqual(ShapeTarget(path: "\(FENCE_POLYGONS).2", line: false), shapeTarget(2, nil))
        XCTAssertNil(shapeTarget(nil, nil))
        let corridor = Survey(index: 4, area: [], transects: [], cameraShots: 0, kind: "", shape: "", property: CORRIDOR_PROPERTY)
        XCTAssertEqual(ShapeTarget(path: "\(PLAN_ITEMS).4.\(CORRIDOR_PROPERTY)", line: true), shapeTarget(nil, corridor))
    }

    func testAFilePolygonIsReadAndAnythingElseIsRefused() {
        let area = JSON.parse(#"{"shape":"polygon","error":"","points":[{"latitude":1.0,"longitude":2.0},{"latitude":3.0,"longitude":4.0}]}"#)
        let polygon = ShapeTarget(path: "p", line: false)
        let polyline = ShapeTarget(path: "l", line: true)
        let read = fileShape(area, polygon)
        XCTAssertEqual([TrackPoint(1.0, 2.0), TrackPoint(3.0, 4.0)], read.0)
        XCTAssertEqual("", read.1)
        XCTAssertEqual("No polylines found in file", fileShape(area, polyline).1)
        XCTAssertEqual("No polygons found in file", fileShape(JSON.parse(#"{"shape":"polyline","error":""}"#), polygon).1)
        XCTAssertEqual("bad coordinate: x", fileShape(JSON.parse(#"{"valid":false,"error":"bad coordinate: x"}"#), polygon).1)
        XCTAssertEqual("No polygons found in file", fileShape(nil, polygon).1)
    }

    func testATraceClosesOnceItIsAPolygon() {
        XCTAssertEqual(Array(small.prefix(2)), traceOutline(Array(small.prefix(2))))
        XCTAssertEqual(Array(small.prefix(3)) + [small[0]], traceOutline(Array(small.prefix(3))))
        XCTAssertEqual(Array(small.prefix(3)), traceOutline(Array(small.prefix(3)), true))
    }

    func testADefaultLineRunsDownTheMiddleOfTheView() {
        let line = defaultLine(small)
        XCTAssertEqual(8.5, line[0].longitude, accuracy: 1e-9)
        XCTAssertEqual(47.4005, line[0].latitude, accuracy: 1e-9)
        XCTAssertEqual(47.3995, line[1].latitude, accuracy: 1e-9)
    }

    func testAShapefilePickedWithItsPrjIsReadThroughTheShp() {
        XCTAssertEqual("shp", mainShapeExtension(["area.prj", "area.SHP"]))
        XCTAssertEqual("kml", mainShapeExtension(["area.kml"]))
        XCTAssertNil(mainShapeExtension([]))
    }

    func testATypedCircleRadiusIsInTheAppDistanceUnitsLikeQgcsSetRadiusDialog() throws {
        XCTAssertEqual(30.48, try XCTUnwrap(circleRadiusMetres("100", 0.3048)), accuracy: 1e-9)
        XCTAssertEqual(12.5, try XCTUnwrap(circleRadiusMetres("12,5", 1.0)), accuracy: 1e-9)
        XCTAssertNil(circleRadiusMetres("0", 0.3048))
        XCTAssertNil(circleRadiusMetres("wide", 1.0))
    }

    func testTheShapeToolbarCaptionFollowsQgcsPolygonAndTraceCaptions() {
        let shape = EditableShape(path: "p", midpoints: [], splitInvokable: "", canRemoveVertex: false, caption: "1.2 ha \u{00B7} 440 m", circleCaption: "Radius 50.0 m")
        XCTAssertEqual("Radius 50.0 m", shapeCaption(shape, true))
        XCTAssertEqual("1.2 ha \u{00B7} 440 m", shapeCaption(shape, false))
        let bare = withChanges(shape) { $0.circleCaption = "" }
        XCTAssertEqual("1.2 ha \u{00B7} 440 m", shapeCaption(bare, true))
        XCTAssertEqual("Click the map to add points \u{00B7} 2 of 3", traceCaption(2, 3))
        XCTAssertEqual("3 points", traceCaption(3, 3))
    }

    func testAShapefileIsStagedWithItsSidecarsAndTheMainFileIsTheShp() throws {
        let folder = try scratchFolder()
        let sources = try scratchFolder()
        let stale = folder.appendingPathComponent("shape.kml")
        try Data("old".utf8).write(to: stale)
        let shp = sources.appendingPathComponent("Field.SHP")
        let prj = sources.appendingPathComponent("Field.prj")
        try Data("geometry".utf8).write(to: shp)
        try Data("projection".utf8).write(to: prj)
        let staged = try XCTUnwrap(stageShapeFiles([prj, shp], folder))
        XCTAssertEqual("shape.shp", staged.lastPathComponent)
        XCTAssertEqual("geometry", try String(contentsOf: staged, encoding: .utf8))
        XCTAssertEqual("projection", try String(contentsOf: folder.appendingPathComponent("shape.prj"), encoding: .utf8))
        XCTAssertFalse(FileManager.default.fileExists(atPath: stale.path))
    }

    func testAFileWithoutAnExtensionStagesUnderItsBareName() throws {
        let folder = try scratchFolder()
        let bare = try scratchFolder().appendingPathComponent("outline")
        try Data("points".utf8).write(to: bare)
        XCTAssertEqual("shape.", try XCTUnwrap(stageShapeFiles([bare], folder)).lastPathComponent)
    }

    func testNothingIsStagedWhenAFileCannotBeCopiedOrNoFileIsGiven() throws {
        let folder = try scratchFolder()
        XCTAssertNil(stageShapeFiles([folder.appendingPathComponent("missing.shp")], folder))
        XCTAssertNil(stageShapeFiles([], folder))
    }

    func testATraceDrawsItsOutlineAndADotPerPoint() {
        let points = [TrackPoint(47.0, 8.0), TrackPoint(47.001, 8.0), TrackPoint(47.001, 8.001)]
        XCTAssertEqual(5, traceFeatures(points, false).count)
        XCTAssertEqual(4, traceFeatures(points, true).count)
        XCTAssertEqual(1, traceFeatures(Array(points.prefix(1)), false).count)
        XCTAssertTrue(traceFeatures([], false).isEmpty)
    }

    private func scratchFolder() throws -> URL {
        let folder = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        addTeardownBlock { try? FileManager.default.removeItem(at: folder) }
        return folder
    }
}
