package one.aircast.map

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

    @Test
    fun `a radius edit redraws the loiter ring with a drag handle due east, never below the circle minimum`() {
        val committed = GotoLocation(TrackPoint(47.4, 8.5), 80.0, "80 m", loiterClockwise = true)
        val edit = LoiterEdit(152.4, clockwise = false, unit = "ft", metresPerUnit = 0.3048)
        val shown = editedGoto(committed, edit)
        assertEquals(GotoLocation(TrackPoint(47.4, 8.5), 152.4, "500 ft", loiterClockwise = false), shown)
        assertEquals(committed, editedGoto(committed, null))
        assertEquals(committed.copy(loiterRadiusMetres = null), editedGoto(committed.copy(loiterRadiusMetres = null), edit))
        val handle = gotoRadiusHandle(shown)!!
        assertEquals(152.4, metresBetween(committed.at, handle), 0.5)
        assertTrue(handle.longitude > committed.at.longitude)
        assertEquals(47.4, handle.latitude, 1e-4)
        assertEquals(152.4, draggedGotoRadius(committed, handle), 0.5)
        assertEquals(MINIMUM_CIRCLE_RADIUS_METRES, draggedGotoRadius(committed, committed.at), 0.0)
        assertNull(gotoRadiusHandle(committed.copy(loiterRadiusMetres = null)))
    }
}
