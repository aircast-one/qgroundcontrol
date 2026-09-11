package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class MarkerDensityTest {

    @Test
    fun `an ordinary plan is not crowded and is drawn exactly as before`() {
        assertFalse(crowded(4))
        assertFalse(crowded(CROWDED_ITEMS))
        assertEquals(MARKER_RADIUS, markerRadius(crowded = false, selected = false), 0.0)
        assertEquals("7", waypointLabel(7, crowded = false))
    }

    @Test
    fun `a mission downloaded off a vehicle is crowded and drops to dots`() {
        assertTrue(crowded(CROWDED_ITEMS + 1))
        assertTrue(crowded(212))
        assertEquals(CROWDED_RADIUS, markerRadius(crowded = true, selected = false), 0.0)
    }

    @Test
    fun `numbering is dropped only when there are too many to read`() {
        assertEquals("", waypointLabel(150, crowded = true))
        assertEquals("150", waypointLabel(150, crowded = false))
    }

    @Test
    fun `the selected item stays full size so it can still be found in a crowd`() {
        assertEquals(MARKER_RADIUS, markerRadius(crowded = true, selected = true), 0.0)
    }

    @Test
    fun `a selected item keeps its heavy outline whatever the crowd`() {
        assertEquals(
            markerStroke(crowded = false, selected = true),
            markerStroke(crowded = true, selected = true),
            0.0,
        )
        assertTrue(
            markerStroke(crowded = true, selected = false) <
                markerStroke(crowded = false, selected = false),
        )
    }

    @Test
    fun `every waypoint carries its own size, so the layer needs no zoom rule`() {
        val items = (0..80).map {
            MissionItem(it, it, 41.0 + it * 0.001, 44.0, "Waypoint", false, kind = "waypoint")
        }
        val features = missionFeatures(items, selectedIndex = 3).features()!!
        assertEquals(81, features.size.toLong().toInt())
        assertEquals(CROWDED_RADIUS, features[0].getNumberProperty(WAYPOINT_RADIUS_PROPERTY).toDouble(), 0.0)
        assertEquals(MARKER_RADIUS, features[3].getNumberProperty(WAYPOINT_RADIUS_PROPERTY).toDouble(), 0.0)
        assertEquals("", features[0].getStringProperty(WAYPOINT_LABEL_PROPERTY))
    }
}
