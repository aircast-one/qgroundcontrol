package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WaypointSpeedTest {
    @Test
    fun `a blank field clears the speed, a positive number sets it, anything else is refused`() {
        assertEquals(SpeedEntry.Clear, speedEntry("  "))
        assertEquals(SpeedEntry.Set(8.5), speedEntry("8,5"))
        assertEquals(SpeedEntry.Invalid, speedEntry("0"))
        assertEquals(SpeedEntry.Invalid, speedEntry("fast"))
    }

    @Test
    fun `the field shows the item's speed only when the item specifies one`() {
        val served = JSONObject("""{"speedSection":{"available":true,"specified":true,"value":8.0,"units":"m/s","path":"p","specifyPath":"s"}}""")
        val speed = waypointSpeed(served)!!
        assertEquals("8", speedFieldText(speed))
        assertEquals("", speedFieldText(speed.copy(specified = false)))
        assertNull(waypointSpeed(JSONObject("""{"speedSection":{"available":false}}""")))
    }

    @Test
    fun `a waypoint's hold is seconds from zero, an empty field meaning no hold`() {
        assertEquals(WaypointHold(5.0, "s", "p"), waypointHold(JSONObject("""{"hold":{"value":5.0,"units":"s","path":"p"}}""")))
        assertNull(waypointHold(JSONObject("""{"hold":null}""")))
        assertEquals(0.0, holdEntry(" ")!!, 0.0)
        assertEquals(2.5, holdEntry("2,5")!!, 0.0)
        assertNull(holdEntry("-1"))
        assertNull(holdEntry("long"))
    }
}
