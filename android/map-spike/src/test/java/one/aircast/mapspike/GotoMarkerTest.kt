package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GotoMarkerTest {
    @Test
    fun `the go here marker carries a loiter ring only when the core sends a radius`() {
        val fixedWing = gotoLocation(JSONObject("""{"gotoLocation":{"latitude":47.4,"longitude":8.5,"loiterRadiusMetres":80.0}}"""))
        assertEquals(GotoLocation(TrackPoint(47.4, 8.5), 80.0), fixedWing)
        val ring = gotoRing(fixedWing)
        assertEquals(ring.first(), ring.last())
        assertTrue(ring.size > 3)
        val copter = gotoLocation(JSONObject("""{"gotoLocation":{"latitude":47.4,"longitude":8.5,"loiterRadiusMetres":null}}"""))
        assertEquals(GotoLocation(TrackPoint(47.4, 8.5), null), copter)
        assertTrue(gotoRing(copter).isEmpty())
        assertNull(gotoLocation(JSONObject("""{"gotoLocation":null}""")))
    }
}
