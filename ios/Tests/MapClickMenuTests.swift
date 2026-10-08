import XCTest
@testable import Aircast

final class MapClickMenuTests: XCTestCase {
    func testReadsTheOfferedActionsInTheOrderTheCoreServesThem() {
        let actions = mapClickActions(JSON.parse(
            #"{"actions":[{"id":"GoTo","path":"vehicle.guidedModeGotoLocation","label":"Go to location","title":"Go To Location","message":"Move the vehicle to the specified location","confirm":true},"#
                + #"{"path":"vehicle.guidedModeROI","label":"ROI at location","title":"ROI","message":"m","confirm":false}]}"#
        ))
        XCTAssertEqual(actions.map(\.label), ["Go to location", "ROI at location"])
        XCTAssertTrue(actions[0].confirm)
        XCTAssertFalse(actions[1].confirm)
        XCTAssertTrue(mapClickActions(nil).isEmpty)
    }

    func testAnOrbitSendsThePointRadiusDirectionAndHeightAboveHomeInThatOrder() {
        let args = orbitArgs(MapPoint(latitude: 47.4, longitude: 8.5), OrbitChoice(radiusMetres: 30.0, clockwise: false, aboveHomeMetres: 50.0))
        XCTAssertEqual(JSON(args), JSON.parse("[47.4,8.5,30.0,false,50.0]"))
        let feet = orbitDefaults(JSON.parse(#"{"orbitDefaultRadius":98.4252,"orbitRadiusUnit":"ft","orbitMetresPerUnit":0.3048,"orbitClockwise":true}"#))
        XCTAssertEqual(radiusMetres("", feet)!, 30.0, accuracy: 1e-3)
        XCTAssertEqual(radiusMetres("50", feet)!, 15.24, accuracy: 1e-9)
        XCTAssertNil(radiusMetres("wide", feet))
    }

    func testOrbitAtLocationOpensADefaultCircleOnTheTappedPointThatTheRadiusFieldEditsInAppUnits() {
        let feet = orbitDefaults(JSON.parse(#"{"orbitDefaultRadius":98.4252,"orbitRadiusUnit":"ft","orbitMetresPerUnit":0.3048,"orbitClockwise":true}"#))
        let opened = orbitOpened(MapPoint(latitude: 47.4, longitude: 8.5), feet)
        XCTAssertEqual(opened.centre, TrackPoint(latitude: 47.4, longitude: 8.5))
        XCTAssertEqual(opened.radiusMetres, 30.0, accuracy: 1e-3)
        XCTAssertTrue(opened.clockwise)
        let edit = orbitEdit(OrbitCircle(centre: opened.centre, radiusMetres: 15.24, clockwise: false), feet)
        XCTAssertEqual(loiterRadiusField(nil, edit), "50")
        XCTAssertEqual(loiterTyped("100", edit).radiusMetres, 30.48, accuracy: 1e-9)
        XCTAssertEqual(orbitOpened(MapPoint(latitude: 47.4, longitude: 8.5), orbitDefaults(nil)).radiusMetres, MINIMUM_CIRCLE_RADIUS_METRES, accuracy: 0)
    }

    func testShowsThePointToSixPlacesAsQgcDoes() {
        XCTAssertEqual(coordinateLines(MapPoint(latitude: 47.397, longitude: 8.5451234)), ["Lat: 47.397000", "Lon: 8.545123"])
    }

    func testATappedItemJumpsToItsSequenceNeverBeforeTheFirstWaypoint() {
        XCTAssertEqual(waypointTarget(0), 1)
        XCTAssertEqual(waypointTarget(4), 4)
        XCTAssertEqual(setWaypointMessage(4), "Adjust current waypoint to 4")
    }

    func testALoiterChangeReSendsTheGotoPointWithTheDirectionAsTheSign() {
        let offer = loiterOffer(JSON.parse(#"{"loiter":{"latitude":47.4,"longitude":8.5,"title":"Change Loiter Radius","message":"m","defaultRadius":80,"clockwise":true}}"#))!
        XCTAssertEqual(offer.latitude, 47.4, accuracy: 0)
        XCTAssertEqual(offer.defaultRadius, 80.0, accuracy: 0)
        XCTAssertEqual(signedLoiterRadius(80.0, false), -80.0, accuracy: 0)
        XCTAssertEqual(signedLoiterRadius(-80.0, true), 80.0, accuracy: 0)
        XCTAssertNil(loiterOffer(JSON.parse(#"{"loiter":null}"#)))
    }

    func testTheLoiterRadiusFieldFollowsTheMapDragButKeepsWhatTheOperatorIsTyping() {
        let offer = loiterOffer(JSON.parse(#"{"loiter":{"latitude":47.4,"longitude":8.5,"title":"t","message":"m","defaultRadius":250,"clockwise":false}}"#))!
        let opened = loiterEditOpened(offer, OrbitDefaults(radius: 0.0, unit: "ft", metresPerUnit: 0.3048, clockwise: true))
        XCTAssertEqual(opened.radiusMetres, 76.2, accuracy: 1e-9)
        XCTAssertFalse(opened.clockwise)
        XCTAssertEqual(loiterRadiusField(nil, opened), "250")
        XCTAssertEqual(loiterRadiusField("250.", opened), "250.")
        XCTAssertEqual(loiterRadiusField("", opened), "")
        let dragged = LoiterEdit(radiusMetres: 100 * 0.3048, clockwise: opened.clockwise, unit: opened.unit, metresPerUnit: opened.metresPerUnit)
        XCTAssertEqual(loiterRadiusField("250", dragged), "100")
        XCTAssertEqual(loiterTyped("100", opened).radiusMetres, 30.48, accuracy: 1e-9)
        XCTAssertEqual(loiterTyped("abc", opened), opened)
        XCTAssertEqual(loiterTyped("0", opened), opened)
    }

    func testGoHereReadsHowFarAndWhichWayThePointLiesInTheOperatorsUnit() {
        let from = MapPoint(latitude: 47.0, longitude: 8.0)
        XCTAssertEqual(goHereText(from, MapPoint(latitude: 47.001, longitude: 8.0), "m", 1.0), "111 m north")
        XCTAssertEqual(goHereText(from, MapPoint(latitude: 47.001, longitude: 8.0), "ft", 0.3048), "365 ft north")
        XCTAssertEqual(goHereText(from, MapPoint(latitude: 47.0, longitude: 8.001), "m", 1.0), "76 m east")
        XCTAssertNil(goHereText(from, MapPoint(latitude: 47.001, longitude: 8.0), "", 1.0))
    }
}
