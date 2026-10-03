package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class FenceRowsTest {
    @Test
    fun `polygons then circles, named like GeoFenceEditor rows with Inclusion or Exclusion beneath`() {
        val polygon = FencePolygon(0, inclusion = false, vertices = emptyList(), detailText = "4 corners")
        val circle = FenceCircle(0, inclusion = true, centre = TrackPoint(0.0, 0.0), radius = 137.0, detailText = "137 m radius", kindText = "Circle 1", radiusUnits = "m")

        assertEquals(
            listOf(FenceRow(0, false, "Polygon 1", "Exclusion", inclusion = false), FenceRow(0, true, "Circle 1", "Inclusion", radius = "137 m")),
            fenceRows(listOf(polygon), listOf(circle)),
        )
    }

    @Test
    fun `rally points are numbered from one with their height and position`() {
        val point = RallyPoint(1, 41.7151, 44.8271, altitude = 50.0, altitudeUnits = "m")

        assertEquals(listOf(FenceRow(1, false, "Rally 2", "50 m \u00b7 41.715100, 44.827100")), rallyRows(listOf(point)))
        assertEquals("41.715100, 44.827100", rallyRows(listOf(point.copy(altitude = Double.NaN))).single().detail)
    }

    @Test
    fun `selecting a fence or rally shape opens its layer, a waypoint the mission's`() {
        assertEquals(PlanLayer.Fence, layerOf(MapHit.Circle(0)))
        assertEquals(PlanLayer.Fence, layerOf(MapHit.FenceVertex(0, 1)))
        assertEquals(PlanLayer.Rally, layerOf(MapHit.Rally(0)))
        assertEquals(PlanLayer.Mission, layerOf(MapHit.Waypoint(2)))
        assertEquals(PlanLayer.Mission, layerOf(MapHit.ShapeCentre(fence = false, owner = 3)))
        assertEquals(null, layerOf(null))
    }
}
