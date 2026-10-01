package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
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
}
