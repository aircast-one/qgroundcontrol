import XCTest
@testable import Aircast

final class ObstacleRingTests: XCTestCase {
    private func ring(_ nonNull: [Int: Double], size: Int = 72) -> String {
        (0..<size).map { nonNull[$0].map { String($0) } ?? "null" }.joined(separator: ",")
    }

    private func served(
        available: Bool = true,
        stale: Bool = false,
        increment: String = "5.0",
        offset: String = "0.0",
        max: String = "40.0",
        body: String? = nil
    ) -> JSON {
        let samples = body ?? ring([18: 3.2])
        return JSON.parse(
            #"{"kind":"object","class":"ObstacleDistance","available":\#(available),"stale":\#(stale),"ringMetres":[\#(samples)],"ringIncrement":\#(increment),"ringOffset":\#(offset),"rangeMinMetres":0.2,"rangeMaxMetres":\#(max)}"#
        )
    }

    func testASectorsIndexPlacesItAndItAgreesWithNearest() throws {
        let read = try XCTUnwrap(obstacleRing(served()))

        XCTAssertEqual(read.samples.count, 1)
        XCTAssertEqual(
            read.samples[0].bearingDegrees,
            90.0,
            accuracy: 1e-9,
            "measured on the rig: sector 18 at 5 degrees a step from an offset of zero is due right, which is exactly what nearest.bearing reported on the same read"
        )
        XCTAssertEqual(read.samples[0].metres, 3.2, accuracy: 1e-9)
    }

    func testAnEmptySectorIsAbsentNotAnObstacleAtZero() throws {
        let read = try XCTUnwrap(obstacleRing(served(body: ring([0: 0.0, 36: 7.5]))))

        XCTAssertEqual(read.samples.count, 2)
        XCTAssertEqual(
            read.samples.map { [$0.bearingDegrees, $0.metres] },
            [[0.0, 0.0], [180.0, 7.5]],
            "zero is a real reading - the sensor saw something at the aircraft - so it is kept, while the seventy nulls around it are dropped"
        )
    }

    func testARingWithNoSpacingCannotBePlacedSoItIsNotDrawn() {
        XCTAssertNil(
            obstacleRing(served(increment: "null")),
            "without an increment a sample has no angle; drawing it anywhere would invent a direction the vehicle never reported"
        )
    }

    func testNothingIsDrawnWhenTheViewSaysThereIsNothingToDraw() {
        XCTAssertNil(obstacleRing(served(available: false)))
        XCTAssertNil(obstacleRing(nil))
        XCTAssertNil(obstacleRing(served(body: ring([:]))), "every sector empty is no arc")
    }

    func testAStaleRingStillHasItsSamplesAndSaysItIsStale() throws {
        let read = try XCTUnwrap(obstacleRing(served(stale: true)))

        XCTAssertTrue(
            read.stale,
            "measured at t+111s: the ring, nearest, sectors and available were all unchanged 67 seconds after the sensor stopped - stale is the only thing that moved, so the drawing has to carry it or the arc claims a live obstacle"
        )
        XCTAssertEqual(read.samples.count, 1)
    }

    func testALiveRingIsNotMarkedStale() {
        XCTAssertNotNil(obstacleRing(served()))
        XCTAssertEqual(obstacleRing(served())?.stale, false)
    }

    func testTheCeilingComesFromTheVehicleNotFromAGuess() throws {
        XCTAssertEqual(try XCTUnwrap(obstacleRing(served())).maxMetres, 40.0, accuracy: 1e-9)
        XCTAssertNil(obstacleRing(served(max: "null")), "no ceiling means no scale to draw against")
    }

    func testTheClosestObstacleIsNeverTheLeastVisible() {
        let ceiling = 40.0

        XCTAssertTrue(
            arcRadiusFraction(3.2, ceiling) >= NEAREST_VISIBLE_FRACTION,
            "measured on the rig: 3.2 m against a 40 m ceiling is 8 percent of the radius, which on a 48dp widget is five pixels from the centre dot - the obstacle an operator most needs to see drawn as nothing at all"
        )
        XCTAssertTrue(
            arcRadiusFraction(30.0, ceiling) > arcRadiusFraction(3.2, ceiling),
            "and it still grows with distance, so the picture stays honest about which is further"
        )
        XCTAssertEqual(arcRadiusFraction(40.0, ceiling), 1, accuracy: 1e-6, "the ceiling is the rim")
        XCTAssertEqual(arcRadiusFraction(90.0, ceiling), 1, accuracy: 1e-6, "beyond the ceiling is still the rim")
        XCTAssertEqual(arcRadiusFraction(3.2, 0.0), 0, accuracy: 1e-6, "no ceiling, nothing to scale against")
    }

    func testTheNearBandDoesNotCollapseWhereTheDifferenceMattersMost() {
        let ceiling = 40.0

        XCTAssertTrue(
            arcRadiusFraction(3.2, ceiling) > arcRadiusFraction(0.2, ceiling) * 2,
            "a linear scale put 0.2 m and 3.2 m at 0.5 and 8 percent, and a floor then made them the same mark - 0.2 m is a strike and 3.2 m is a manoeuvre, so that is the one place the picture must not flatten"
        )
        XCTAssertTrue(arcRadiusFraction(1.0, ceiling) > arcRadiusFraction(0.2, ceiling))
    }

    func testTooNearIsTheVehiclesOwnJudgementNotAPixelThreshold() {
        XCTAssertTrue(
            sampleIsClose(0.2, 0.2),
            "inside twice the sensor's rated floor is the vehicle saying too near, and it survives wherever the radius has to floor out"
        )
        XCTAssertFalse(sampleIsClose(0.5, 0.2))
        XCTAssertFalse(
            sampleIsClose(0.1, 0.0),
            "with no rated floor nothing is near, and it needs no guard of its own: the ring drops any sample below zero, so a distance can never be under twice nothing"
        )
    }

    func testTheRingCarriesTheSpacingAndFloorItWasMeasuredWith() throws {
        let read = try XCTUnwrap(obstacleRing(served()))

        XCTAssertEqual(
            read.incrementDegrees,
            5.0,
            accuracy: 1e-9,
            "the arc used to re-read these from the view with defaults of 5 degrees and zero metres, which invent a spacing the vehicle never reported and a floor that makes nothing ever close"
        )
        XCTAssertEqual(read.floorMetres, 0.2, accuracy: 1e-9)
    }

    func testAVehicleReportingNoFloorHasNothingJudgedClose() throws {
        let noFloor = JSON.parse(
            #"{"kind":"object","class":"ObstacleDistance","available":true,"stale":false,"ringMetres":[\#(ring([18: 3.2]))],"ringIncrement":5.0,"ringOffset":0.0,"rangeMinMetres":null,"rangeMaxMetres":40.0}"#
        )
        let read = try XCTUnwrap(obstacleRing(noFloor))

        XCTAssertEqual(read.floorMetres, 0.0, accuracy: 1e-9)
        XCTAssertFalse(
            sampleIsClose(read.samples[0].metres, read.floorMetres),
            "no rated floor means the vehicle has not said what too near is, so nothing is"
        )
    }

    func testACloseReadingNeverLooksLikeADistantOne() {
        XCTAssertNotEqual(
            arcTone(true, false),
            arcTone(false, false),
            "the arc drew both in the same colour while live, because the near branch and the default branch both resolved to the error colour - a distinction that was only ever visible when the reading was stale, which is when it matters least"
        )
    }

    func testAStaleReadingLooksLikeNeither() {
        XCTAssertEqual(arcTone(true, true), .Stale)
        XCTAssertEqual(arcTone(false, true), .Stale)
        XCTAssertNotEqual(arcTone(true, false), .Stale)
        XCTAssertNotEqual(arcTone(false, false), .Stale)
    }

    func testASectorsWedgeIsNarrowerThanItsSpacingSoNeighboursStayApart() {
        XCTAssertTrue(arcSweep(5.0) < 5.0)
        XCTAssertTrue(arcSweep(5.0) > 0)
    }
}
