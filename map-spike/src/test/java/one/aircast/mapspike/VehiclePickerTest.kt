package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class VehiclePickerTest {
    private fun list(vararg ids: Int) =
        JSONObject(
            """{"kind":"object","elements":[${ids.joinToString(",") { """{"id":$it}""" }}]}""",
        )

    @Test
    fun `each connected vehicle is an entry at its list index`() {
        val entries = vehicleEntries(list(1, 2), activeId = 2)

        assertEquals(listOf(0, 1), entries.map { it.index })
        assertEquals(listOf(1, 2), entries.map { it.id })
    }

    @Test
    fun `the active one is marked and the others are not`() {
        val entries = vehicleEntries(list(1, 2), activeId = 2)

        assertEquals(listOf(false, true), entries.map { it.active })
    }

    @Test
    fun `no vehicle is active when none matches`() {
        assertTrue(vehicleEntries(list(1, 2), activeId = -1).none { it.active })
    }

    @Test
    fun `an entry without an id is not offered`() {
        val json = JSONObject("""{"kind":"object","elements":[{"id":1},{}]}""")

        assertEquals(1, vehicleEntries(json, activeId = 1).size)
    }

    @Test
    fun `nothing connected is an empty list rather than a failure`() {
        assertTrue(vehicleEntries(null, activeId = 1).isEmpty())
    }
}
