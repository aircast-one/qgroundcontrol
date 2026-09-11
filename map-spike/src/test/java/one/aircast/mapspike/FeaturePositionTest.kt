package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test
import org.maplibre.geojson.FeatureCollection
import org.maplibre.geojson.LineString
import org.maplibre.geojson.Point
import org.maplibre.geojson.Polygon

class FeaturePositionTest {
    private fun at(latitude: Double, longitude: Double) = TrackPoint(latitude, longitude)

    private fun points(features: FeatureCollection) =
        features.features().orEmpty().map {
            (it.geometry() as Point).let { p -> p.latitude() to p.longitude() }
        }

    private fun ring(features: FeatureCollection) =
        (features.features().orEmpty().first().geometry() as Polygon)
            .coordinates().first().map { it.latitude() to it.longitude() }

    private fun line(features: FeatureCollection) =
        (features.features().orEmpty().first().geometry() as LineString)
            .coordinates().map { it.latitude() to it.longitude() }

    private fun item(index: Int, latitude: Double, longitude: Double) =
        MissionItem(index, index, latitude, longitude, "Waypoint", false)

    @Test
    fun `a waypoint marker sits on the waypoint`() {
        assertEquals(
            listOf(41.0 to 44.0, 41.2 to 44.3),
            points(missionFeatures(listOf(item(0, 41.0, 44.0), item(1, 41.2, 44.3)))),
        )
    }

    @Test
    fun `a rally marker sits on the rally point`() {
        assertEquals(
            listOf(41.5 to 44.5),
            points(rallyFeatures(listOf(RallyPoint(0, 41.5, 44.5)))),
        )
    }

    @Test
    fun `the vehicle marker sits on the vehicle`() {
        assertEquals(listOf(41.7 to 44.8), points(vehicleFeatures(41.7, 44.8, 90.0)))
    }

    @Test
    fun `a fence ring runs through its vertices and closes`() {
        val fence = FencePolygon(0, true, listOf(at(41.0, 44.0), at(41.0, 44.1), at(41.1, 44.1)))

        assertEquals(
            listOf(41.0 to 44.0, 41.0 to 44.1, 41.1 to 44.1, 41.0 to 44.0),
            ring(fenceFeatures(listOf(fence))),
        )
    }

    @Test
    fun `a survey area runs through its corners and closes`() {
        val survey = Survey(0, listOf(at(42.0, 45.0), at(42.0, 45.1), at(42.1, 45.1)), emptyList(), 0)

        assertEquals(
            listOf(42.0 to 45.0, 42.0 to 45.1, 42.1 to 45.1, 42.0 to 45.0),
            ring(surveyAreaFeatures(listOf(survey))),
        )
    }

    @Test
    fun `survey transects run through their points in order`() {
        val survey = Survey(0, emptyList(), listOf(at(43.0, 46.0), at(43.1, 46.1)), 0)

        assertEquals(listOf(43.0 to 46.0, 43.1 to 46.1), line(surveyTransectFeatures(listOf(survey))))
    }
}
