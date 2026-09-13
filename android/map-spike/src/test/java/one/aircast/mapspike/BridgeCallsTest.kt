package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class BridgeCallsTest {
    @Test
    fun `a coordinate carries all three keys, because the bridge needs latitude and longitude`() {
        assertEquals(
            """{"latitude":41.5,"longitude":44.25,"altitude":0}""",
            coordinateJson(41.5, 44.25),
        )
    }

    @Test
    fun `a track point makes the same coordinate as its two numbers`() {
        assertEquals(coordinateJson(41.5, 44.25), coordinateJson(TrackPoint(41.5, 44.25)))
    }

    @Test
    fun `a property write wraps its value, which a bare coordinate does not`() {
        assertEquals(
            """{"value":{"latitude":41.5,"longitude":44.25,"altitude":0}}""",
            settingJson(coordinateJson(41.5, 44.25)),
        )
        assertEquals("""{"value":50.0}""", settingJson("50.0"))
    }

    @Test
    fun `a double is written in a form the bridge parses, whatever the phone's locale`() {
        assertEquals("""{"latitude":-0.5,"longitude":179.125,"altitude":0}""", coordinateJson(-0.5, 179.125))
    }
}
