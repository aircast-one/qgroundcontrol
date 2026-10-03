package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class OrbitMarkerTest {
    @Test
    fun `the circle is drawn only while the vehicle orbits, as FlyViewMap shows orbitMapCircle`() {
        val turning = orbitCircle(JSONObject("""{"class":"Orbit","orbiting":true,"centre":{"latitude":47.0,"longitude":8.0},"radiusMetres":50}"""))
        assertEquals(OrbitCircle(TrackPoint(47.0, 8.0), 50.0), turning)
        assertEquals(orbitRing(turning).first(), orbitRing(turning).last())
        assertNull(orbitCircle(JSONObject("""{"class":"Orbit","orbiting":false,"centre":null}""")))
        assertNull(orbitCircle(null))
        assertEquals(emptyList<TrackPoint>(), orbitRing(null))
    }

    @Test
    fun `an orbit preview has a centre handle and a radius handle due east that drags the radius`() {
        val preview = OrbitCircle(TrackPoint(47.4, 8.5), 30.0, clockwise = true)
        val handles = orbitHandles(preview)
        assertEquals(preview.centre, handles.first())
        assertEquals(30.0, metresBetween(preview.centre, handles.last()), 0.5)
        assertTrue(handles.last().longitude > preview.centre.longitude)
        assertEquals(30.0, draggedOrbitRadius(preview, handles.last()), 0.5)
        assertEquals(MINIMUM_CIRCLE_RADIUS_METRES, draggedOrbitRadius(preview, preview.centre), 0.0)
        assertTrue(orbitHandles(null).isEmpty())
    }
}
