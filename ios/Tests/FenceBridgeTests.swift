import XCTest
@testable import Aircast

final class FenceBridgeTests: XCTestCase {
    private func vertex(_ latitude: Double, _ longitude: Double) -> String { #"{"latitude":\#(latitude),"longitude":\#(longitude)}"# }

    private func polygon(_ vertices: [String], inclusion: Bool = true) -> String {
        #"{"inclusion":\#(inclusion),"vertices":[\#(vertices.joined(separator: ","))]}"#
    }

    private func model(_ elements: String...) -> JSON { JSON.parse(#"{"kind":"object","polygons":[\#(elements.joined(separator: ","))]}"#) }

    private func circles(_ elements: String...) -> JSON { JSON.parse(#"{"kind":"object","circles":[\#(elements.joined(separator: ","))]}"#) }

    private func rally(_ elements: String...) -> JSON { JSON.parse(#"{"kind":"object","rallyPoints":[\#(elements.joined(separator: ","))]}"#) }

    private var square: [String] { [vertex(41.0, 44.0), vertex(41.0, 44.1), vertex(41.1, 44.1), vertex(41.1, 44.0)] }

    func testAClosedPolygonKeepsItsVerticesAndInclusion() {
        let polygons = fencePolygons(model(polygon(square, inclusion: false)))
        XCTAssertEqual(1, polygons.count)
        XCTAssertEqual(4, polygons[0].vertices.count)
        XCTAssertFalse(polygons[0].inclusion)
        XCTAssertEqual(TrackPoint(41.0, 44.0), polygons[0].vertices.first)
    }

    func testAPolygonThatCannotCloseIsNotDrawn() {
        let polygons = fencePolygons(model(polygon([vertex(41.0, 44.0), vertex(41.0, 44.1)]), polygon(square)))
        XCTAssertEqual(1, polygons.count)
    }

    func testUnusableVerticesAreDroppedAndCanSinkAPolygon() {
        XCTAssertEqual(0, fencePolygons(model(polygon([vertex(41.0, 44.0), vertex(0.0, 0.0), vertex(41.1, 44.1)]))).count)
    }

    func testACircleReadsTheRadiusTheCoreResolved() {
        let good = #"{"inclusion":true,"centre":\#(vertex(41.0, 44.0)),"radius":136.0}"#
        let decoded = fenceCircles(circles(good))
        XCTAssertEqual(1, decoded.count)
        XCTAssertEqual(136.0, decoded[0].radius, accuracy: 1e-9)
        XCTAssertTrue(decoded[0].inclusion)
    }

    func testCirclesWithoutACentreOrAUsableRadiusAreDropped() {
        let noRadius = #"{"inclusion":true,"centre":\#(vertex(41.0, 44.0)),"radius":0.0}"#
        let noRadiusField = #"{"inclusion":true,"centre":\#(vertex(41.0, 44.0))}"#
        let noCentre = #"{"inclusion":true,"radius":120.0}"#
        let otherFact = #"{"centre":\#(vertex(41.0, 44.0)),"radius":null}"#
        XCTAssertEqual(0, fenceCircles(circles(noRadius, noRadiusField, noCentre, otherFact)).count)
    }

    func testRallyPointsReadTheirCoordinate() {
        let points = rallyPoints(rally(vertex(41.2, 44.2), vertex(0.0, 0.0), "{}"))
        XCTAssertEqual(1, points.count)
        XCTAssertEqual(41.2, points[0].latitude, accuracy: 1e-9)
        XCTAssertEqual(0, points[0].index)
    }

    func testAnAbsentModelYieldsNothing() {
        XCTAssertEqual(0, fencePolygons(nil).count)
        XCTAssertEqual(0, fenceCircles(nil).count)
        XCTAssertEqual(0, rallyPoints(nil).count)
    }
}
