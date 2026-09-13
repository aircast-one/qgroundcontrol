package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class HomePositionTest {
    private fun coordinate(latitude: Double, longitude: Double, valid: Boolean = true) =
        JSONObject(
            """{"kind":"coordinate","valid":$valid,"latitude":$latitude,"longitude":$longitude}""",
        )

    @Test
    fun `a valid coordinate is read`() {
        val point = coordinateOf(coordinate(-35.363262, 149.165237))

        assertEquals(-35.363262, point!!.latitude, 1e-9)
        assertEquals(149.165237, point.longitude, 1e-9)
    }

    @Test
    fun `a coordinate the vehicle has not established is ignored`() {
        assertNull(coordinateOf(coordinate(0.0, 0.0, valid = false)))
        assertNull(coordinateOf(coordinate(-35.36, 149.16, valid = false)))
    }

    @Test
    fun `null island is not a home position even when flagged valid`() {
        assertNull(coordinateOf(coordinate(0.0, 0.0)))
    }

    @Test
    fun `an absent or unrelated payload yields nothing`() {
        assertNull(coordinateOf(null))
        assertNull(coordinateOf(JSONObject("""{"kind":"null"}""")))
    }
}

