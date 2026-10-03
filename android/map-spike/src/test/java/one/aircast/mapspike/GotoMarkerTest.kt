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

    @Test
    fun `the loiter ring points its rotation arrows the way the circle was committed`() {
        val anticlockwise = gotoLocation(JSONObject("""{"gotoLocation":{"latitude":47.4,"longitude":8.5,"loiterRadiusMetres":80.0,"loiterClockwise":false}}"""))
        assertEquals(listOf(270.0, 90.0), gotoArrows(anticlockwise).map { it.second })
        assertEquals(listOf(90.0, 270.0), gotoArrows(anticlockwise?.copy(loiterClockwise = true)).map { it.second })
        assertTrue(gotoArrows(anticlockwise?.copy(loiterRadiusMetres = null)).isEmpty())
    }
}
