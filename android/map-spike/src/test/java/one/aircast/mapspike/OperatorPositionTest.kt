package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class OperatorPositionTest {
    @Test
    fun `a usable fix is a point on the map`() {
        val view = JSONObject("""{"usable":true,"latitude":41.6983,"longitude":44.85148}""")

        assertEquals(TrackPoint(41.6983, 44.85148), operatorPoint(view))
        assertEquals(1, operatorFeatures(operatorPoint(view)).features()?.size)
    }

    @Test
    fun `a fix the core will not stand behind is not drawn`() {
        val stale = JSONObject("""{"usable":false,"latitude":41.6983,"longitude":44.85148}""")

        assertNull("usable is the core's own gate and the map does not second-guess it", operatorPoint(stale))
        assertNull(operatorPoint(JSONObject("""{"usable":true,"latitude":null,"longitude":null}""")))
        assertNull(operatorPoint(null))
        assertEquals(0, operatorFeatures(null).features()?.size)
    }
}
