package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WaypointSpeedTest {
    @Test
    fun `the speed row reads the item's speed section and skips a vehicle that has none`() {
        val served = JSONObject("""{"speedSection":{"available":true,"specified":true,"value":8.0,"units":"m/s","path":"p","specifyPath":"s"}}""")
        assertEquals(WaypointSpeed(true, 8.0, "m/s", "p", "s"), waypointSpeed(served))
        assertNull(waypointSpeed(JSONObject("""{"speedSection":{"available":false}}""")))
    }

    @Test
    fun `a waypoint's hold is read in seconds, and an item without one shows no row`() {
        assertEquals(WaypointHold(5.0, "s", "p"), waypointHold(JSONObject("""{"hold":{"value":5.0,"units":"s","path":"p"}}""")))
        assertNull(waypointHold(JSONObject("""{"hold":null}""")))
    }
}
