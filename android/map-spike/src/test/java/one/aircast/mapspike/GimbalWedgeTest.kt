package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class GimbalWedgeTest {
    @Test
    fun `each gimbal's yaw is read for the map wedge`() {
        val read = gimbalAzimuths(JSONObject("""{"gimbals":[{"yaw":270.0,"active":true},{"yaw":10.0,"active":false}]}"""))
        assertEquals(listOf(GimbalAzimuth(270.0, true), GimbalAzimuth(10.0, false)), read)
        assertEquals(emptyList<GimbalAzimuth>(), gimbalAzimuths(null))
    }
}
