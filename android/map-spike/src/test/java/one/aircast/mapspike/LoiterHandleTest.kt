package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class LoiterHandleTest {

    private val loiter = MissionItem(index = 2, sequence = 2, latitude = 47.0, longitude = 8.0, command = "", selected = false, loiterRadius = 80.0)
    private val waypoint = MissionItem(index = 1, sequence = 1, latitude = 47.1, longitude = 8.0, command = "", selected = false)

    @Test
    fun `the arrows run with the turn, the way QGCMapCircleVisuals points them`() {
        assertEquals(listOf(90.0, 270.0), loiterRotationArrows(listOf(waypoint, loiter)).map { it.bearing })
        assertEquals("counter-clockwise flips them", listOf(270.0, 90.0), loiterRotationArrows(listOf(loiter.copy(loiterRadius = -80.0))).map { it.bearing })
    }

    @Test
    fun `only the current loiter gets a radius handle, every loiter gets its arrows`() {
        val kinds = { selected: Int? -> loiterHandleFeatures(listOf(waypoint, loiter), selected).map { it.getStringProperty(HANDLE_KIND_PROPERTY) } }
        assertEquals(listOf(HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_ROTATION), kinds(null))
        assertEquals(listOf(HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_ROTATION, HANDLE_KIND_LOITER_RADIUS), kinds(2))
        assertEquals(MapHit.LoiterRadius(2), handleHit(HANDLE_KIND_LOITER_RADIUS, 2, 0))
        assertEquals(MapHit.LoiterRotation(2), handleHit(HANDLE_KIND_LOITER_ROTATION, 2, 1))
    }

    @Test
    fun `dragging the radius keeps the direction it turns`() {
        val to = pointAt(TrackPoint(47.0, 8.0), 150.0, 90.0)
        assertEquals(150.0, draggedLoiterRadius(loiter, to), 0.5)
        assertEquals(-150.0, draggedLoiterRadius(loiter.copy(loiterRadius = -80.0), to), 0.5)
    }
}

class ClickMarkerTest {

    @Test
    fun `the marker sits on the tapped point and nowhere once the menu closes`() {
        assertEquals(1, clickMarkerFeatures(TrackPoint(47.0, 8.0)).features()!!.size)
        assertEquals(0, clickMarkerFeatures(null).features()!!.size)
        assertEquals(0, clickMarkerFeatures(TrackPoint(Double.NaN, 8.0)).features()!!.size)
    }
}
