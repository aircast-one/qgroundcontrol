import MapLibre
import XCTest
@testable import Aircast

final class FleetFeaturesTests: XCTestCase {
    private func fleet(_ json: String) -> [VehicleChoice] { vehicleChoices(JSON.parse(json)).choices }

    private lazy var two = fleet(
        """
        {"ambiguous":true,"vehicles":[
          {"id":1,"name":"Quadrotor 1","active":true,"contactLost":false,"heading":90.0,
           "home":{"latitude":41.69,"longitude":-74.01},
           "coordinate":{"latitude":41.7,"longitude":-74.0}},
          {"id":2,"name":"Quadrotor 2","active":false,"contactLost":true,"heading":null,"home":null,
           "coordinate":{"latitude":41.8,"longitude":-74.1}}
        ]}
        """
    )

    func testWithMoreThanOneAircraftEachIsLabelledWithItsSystemId() {
        let drawn = fleetFeatures(two).shapes
        XCTAssertEqual(drawn[0].getStringProperty(VEHICLE_LABEL_PROPERTY), "Vehicle 1")
        XCTAssertEqual(drawn[1].getStringProperty(VEHICLE_LABEL_PROPERTY), "Vehicle 2")
        XCTAssertFalse(fleetFeatures(Array(two.prefix(1))).shapes[0].hasProperty(VEHICLE_LABEL_PROPERTY), "VehicleMapItem hides the label with a single vehicle")
    }

    func testASilentAircraftKeepsItsLastPlaceOnTheMapAndSaysHowLongAgoThatWas() {
        let alone = fleetFeatures(Array(two.prefix(1)), lastSeen: lastSeenText(7)).shapes[0]
        XCTAssertEqual(alone.getStringProperty(VEHICLE_LABEL_PROPERTY), "Last seen 7 s ago")
        let drawn = fleetFeatures(two, lastSeen: lastSeenText(90)).shapes
        XCTAssertEqual(drawn[0].getStringProperty(VEHICLE_LABEL_PROPERTY), "Vehicle 1\nLast seen 1 min ago")
        XCTAssertEqual(drawn[1].getStringProperty(VEHICLE_LABEL_PROPERTY), "Vehicle 2", "only the aircraft being flown carries the flown link's silence")
    }

    func testEveryAircraftIsDrawnNotOnlyTheOneBeingFlown() throws {
        let drawn = fleetFeatures(two).shapes
        XCTAssertEqual(drawn.count, 2)
        XCTAssertEqual(try XCTUnwrap(drawn[0] as? MLNPointFeature).coordinate.longitude, -74.0, accuracy: 1e-9)
        XCTAssertEqual(try XCTUnwrap(drawn[1] as? MLNPointFeature).coordinate.longitude, -74.1, accuracy: 1e-9)
    }

    func testEachAircraftCarriesItsOwnHeadingAndOneWithNoneIsNotDrawnPointingNorth() throws {
        let drawn = fleetFeatures(two).shapes
        XCTAssertEqual(try XCTUnwrap(drawn[0].getNumberProperty(HEADING_PROPERTY)), 90.0, accuracy: 1e-9)
        XCTAssertFalse(drawn[1].hasProperty(HEADING_PROPERTY), "view.vehicles serves null before an attitude arrives, and an arrow would claim north")
    }

    func testEachAircraftCarriesItsOwnHomeAndAnInvalidOneIsNone() {
        XCTAssertEqual(two[0].home, TrackPoint(latitude: 41.69, longitude: -74.01))
        XCTAssertNil(two[1].home)
    }

    func testEachAircraftCarriesItsOwnSilenceAndItsOwnPlaceInTheList() {
        let drawn = fleetFeatures(two).shapes
        XCTAssertFalse(drawn[0].getBooleanProperty(STALE_PROPERTY))
        XCTAssertTrue(drawn[1].getBooleanProperty(STALE_PROPERTY), "the second stopped answering and the map should not draw it as live")
        XCTAssertTrue(drawn[0].getBooleanProperty(ACTIVE_PROPERTY))
        XCTAssertFalse(drawn[1].getBooleanProperty(ACTIVE_PROPERTY))
    }

    func testAnAircraftThatHasReportedNoPositionIsLeftOffRatherThanDrawnAtZero() {
        let unplaced = fleet(
            """
            {"vehicles":[
             {"id":1,"name":"A","active":true,"coordinate":{"latitude":41.7,"longitude":-74.0}},
             {"id":2,"name":"B","active":false,"coordinate":null},
             {"id":3,"name":"C","active":false,"coordinate":{"latitude":0,"longitude":0}}]}
            """
        )
        XCTAssertEqual(
            fleetFeatures(unplaced).shapes.count,
            1,
            "null island is where an unset coordinate lands, and a marker there is a lie about an aircraft rather than an absence"
        )
    }

    func testNoFleetDrawsNothingRatherThanAMarkerAtNowhere() {
        XCTAssertEqual(fleetFeatures([]).shapes.count, 0)
    }
}
