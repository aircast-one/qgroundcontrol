package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class FenceRowsTest {
    @Test
    fun `polygons then circles, named by what the core serves or by their kind`() {
        val polygon = FencePolygon(0, inclusion = false, vertices = emptyList(), detailText = "4 corners")
        val circle = FenceCircle(0, inclusion = true, centre = TrackPoint(0.0, 0.0), radius = 137.0, detailText = "137 m radius", kindText = "Keep-in circle")

        assertEquals(
            listOf(FenceRow(0, false, "Keep-out polygon", "4 corners"), FenceRow(0, true, "Keep-in circle", "137 m radius")),
            fenceRows(listOf(polygon), listOf(circle)),
        )
    }
}
