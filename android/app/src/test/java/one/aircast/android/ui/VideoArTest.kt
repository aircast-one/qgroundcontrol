package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VideoArTest {
    @Test
    fun `the core's projection becomes marks and legs on the picture`() {
        val reading = arReading(JSONObject("""{"available":true,"home":{"x":0.2,"up":-0.1,"label":"H","text":"120 m"},"waypoints":[{"x":0.5,"up":0.0,"label":"1"}],"route":[{"from":{"x":0.2,"up":-0.1},"to":{"x":0.5,"up":0.0}}]}"""))!!
        assertEquals(ArMark(ArSpot(0.2f, -0.1f), "H", "120 m"), reading.home)
        assertEquals(listOf("1"), reading.waypoints.map { it.label })
        assertEquals(1, reading.route.size)
        assertNull(arReading(JSONObject("""{"available":false}""")))
    }

    @Test
    fun `the drop target and the predicted impact are read as their own marks`() {
        val reading = arReading(JSONObject("""{"available":true,"aim":{"x":0.5,"up":0.1,"label":"Target","text":"100 m"},"impact":{"x":0.5,"up":0.0,"label":"Drop","text":"50 m"},"waypoints":[],"route":[]}"""))!!
        assertEquals(ArMark(ArSpot(0.5f, 0.1f), "Target", "100 m"), reading.aim)
        assertEquals(ArMark(ArSpot(0.5f, 0.0f), "Drop", "50 m"), reading.impact)
        assertNull(arReading(JSONObject("""{"available":true}"""))!!.aim)
    }

    @Test
    fun `the lead point and the line from the target to it are read`() {
        val reading = arReading(JSONObject("""{"available":true,"aim":{"x":0.5,"up":0.1,"label":"Target","text":"60 m"},"lead":{"x":0.5,"up":0.2},"leadLine":{"from":{"x":0.5,"up":0.1},"to":{"x":0.5,"up":0.2}},"waypoints":[],"route":[]}"""))!!
        assertEquals(ArSpot(0.5f, 0.2f), reading.lead)
        assertEquals(ArSpot(0.5f, 0.1f) to ArSpot(0.5f, 0.2f), reading.leadLine)
        assertNull(arReading(JSONObject("""{"available":true}"""))!!.leadLine)
    }

    @Test
    fun `a spot is placed from the centre line in widths, so the frame's aspect does not distort it`() {
        val centre = arOffset(ArSpot(0.5f, 0f), 1600f, 900f)
        assertEquals(800f, centre.x, 1e-3f)
        assertEquals(450f, centre.y, 1e-3f)
        assertEquals(450f - 160f, arOffset(ArSpot(0.5f, 0.1f), 1600f, 900f).y, 1e-3f)
    }

    @Test
    fun `an orbit outline is drawn only between neighbours both in front of the camera`() {
        val spots = outlineSpots(JSONObject("""{"outline":[{"x":0.1,"up":0.0},{"x":0.2,"up":0.0},null,{"x":0.4,"up":0.0},{"x":0.5,"up":0.0}]}"""))
        assertEquals(5, spots.size)
        assertNull(spots[2])
        assertEquals(
            listOf(ArSpot(0.1f, 0f) to ArSpot(0.2f, 0f), ArSpot(0.4f, 0f) to ArSpot(0.5f, 0f)),
            outlineLegs(spots),
        )
        assertEquals(emptyList<ArSpot?>(), outlineSpots(null))
    }

    @Test
    fun `the running orbit's outline comes with the video overlay, and none when not orbiting`() {
        assertEquals(listOf(ArSpot(0.5f, -0.1f), null), orbitOutline(JSONObject("""{"available":true,"orbit":{"radiusMetres":30.0,"outline":[{"x":0.5,"up":-0.1},null]}}""")))
        assertEquals(emptyList<ArSpot?>(), orbitOutline(JSONObject("""{"available":true,"orbit":null}""")))
    }
}
