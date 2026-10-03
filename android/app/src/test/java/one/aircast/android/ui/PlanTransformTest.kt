package one.aircast.android.ui

import one.aircast.mapspike.TrackPoint
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanTransformTest {
    @Test
    fun `an offset sends east, north, up and the two scope flags`() {
        val metres = DistanceUnit("m", 1.0)
        assertEquals(listOf(10.0, -5.0, 0.0, true, false), offsetArgs("10", "-5", "", metres, metres, takeoff = true, landing = false))
        assertNull(offsetArgs("ten", "0", "0", metres, metres, takeoff = false, landing = false))
    }

    @Test
    fun `offsets typed in feet are sent in metres like the cooked QGC facts`() {
        val view = JSONObject("""{"horizontalUnit":"ft","horizontalMetresPerUnit":0.3048,"verticalUnit":"m","verticalMetresPerUnit":1.0}""")
        val feet = transformUnit(view, "horizontal")
        assertEquals(DistanceUnit("ft", 0.3048), feet)
        assertEquals(listOf(3.048, 0.0, 2.0, false, false), offsetArgs("10", "0", "2", feet, transformUnit(view, "vertical"), takeoff = false, landing = false))
        assertEquals(DistanceUnit("m", 1.0), transformUnit(null, "horizontal"))
    }

    @Test
    fun `home comes from the transform view`() {
        assertEquals(TrackPoint(47.0, 8.0), transformHome(JSONObject("""{"home":{"latitude":47.0,"longitude":8.0}}""")))
        assertNull(transformHome(JSONObject("""{"home":null}""")))
    }
}
