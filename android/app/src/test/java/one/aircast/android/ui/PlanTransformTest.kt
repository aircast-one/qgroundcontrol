package one.aircast.android.ui

import one.aircast.mapspike.TrackPoint
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PlanTransformTest {
    @Test
    fun `an offset sends east, north, up and the two scope flags`() {
        assertEquals(listOf(10.0, -5.0, 0.0, true, false), offsetArgs("10", "-5", "", takeoff = true, landing = false))
        assertNull(offsetArgs("ten", "0", "0", takeoff = false, landing = false))
    }

    @Test
    fun `home comes from the transform view`() {
        assertEquals(TrackPoint(47.0, 8.0), transformHome(JSONObject("""{"home":{"latitude":47.0,"longitude":8.0}}""")))
        assertNull(transformHome(JSONObject("""{"home":null}""")))
    }
}
